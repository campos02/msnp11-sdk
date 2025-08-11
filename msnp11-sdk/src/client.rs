use crate::enums::event::Event;
use crate::enums::msnp_list::MsnpList;
use crate::enums::msnp_status::MsnpStatus;
use crate::event_handler::EventHandler;
use crate::internal_event::InternalEvent;
use crate::models::personal_message::PersonalMessage;
use crate::models::presence::Presence;
use crate::notification_server::commands::{
    adc, adg, blp, chg, cvr, gcf, gtc, prp, reg, rem, rmg, sbp, syn, usr_i, usr_s, uux, ver, xfr,
};
use crate::notification_server::event_matcher::{into_event, into_internal_event};
use crate::passport_auth::PassportAuth;
use crate::receive_split::receive_split;
use crate::sdk_error::SdkError;
use crate::switchboard_server::switchboard::Switchboard;
use crate::user_data::UserData;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use log::{error, trace};
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, lookup_host};
use tokio::sync::{broadcast, mpsc};

/// Defines the client itself, all Notification Server actions are done through an instance of this struct.
pub struct Client {
    event_tx: async_channel::Sender<Event>,
    event_rx: async_channel::Receiver<Event>,
    ns_tx: mpsc::Sender<Vec<u8>>,
    internal_tx: broadcast::Sender<InternalEvent>,
    tr_id: Arc<AtomicU32>,
    user_data: Arc<RwLock<UserData>>,
}

impl Client {
    /// Connects to the server, defines the channels and returns a new instance.
    pub async fn new(server: &str, port: u16) -> Result<Self, SdkError> {
        let mut server_ips = lookup_host((server, port))
            .await
            .or(Err(SdkError::ResolutionError))?;

        let server_ip = server_ips
            .find(|ip| ip.is_ipv4())
            .ok_or(SdkError::ResolutionError)?
            .ip();

        let (event_tx, event_rx) = async_channel::bounded::<Event>(32);
        let (ns_tx, mut ns_rx) = mpsc::channel::<Vec<u8>>(32);
        let (internal_tx, _) = broadcast::channel::<InternalEvent>(64);

        let socket = TcpStream::connect((server_ip, port))
            .await
            .or(Err(SdkError::ServerError))?;

        let (mut rd, mut wr) = socket.into_split();

        let internal_task_tx = internal_tx.clone();
        let event_task_tx = event_tx.clone();
        tokio::spawn(async move {
            'outer: while let Ok(messages) = receive_split(&mut rd).await {
                for message in messages {
                    let internal_event = into_internal_event(&message);
                    if let Err(error) = internal_task_tx.send(internal_event) {
                        error!("{error}");
                    }

                    let event = into_event(&message);
                    if let Some(event) = event {
                        let disconnected =
                            matches!(event, Event::Disconnected | Event::LoggedInAnotherDevice);

                        if let Err(error) = event_task_tx.send(event).await {
                            error!("{error}");
                            break 'outer;
                        }

                        if disconnected {
                            event_task_tx.close();
                        }
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Some(message) = ns_rx.recv().await {
                if let Err(error) = wr.write_all(message.as_slice()).await {
                    error!("{error}");
                }
            }
        });

        Ok(Self {
            event_tx,
            event_rx,
            ns_tx,
            internal_tx,
            tr_id: Arc::new(AtomicU32::new(0)),
            user_data: Arc::new(RwLock::new(UserData::new())),
        })
    }

    fn start_pinging(&self) {
        let event_tx = self.event_tx.clone();
        let ns_tx = self.ns_tx.clone();
        let mut internal_rx = self.internal_tx.subscribe();

        tokio::spawn(async move {
            let command = "PNG\r\n";
            'outer: while ns_tx.send(command.as_bytes().to_vec()).await.is_ok() {
                trace!("C: {command}");

                while let Ok(InternalEvent::ServerReply(reply)) = internal_rx.recv().await {
                    trace!("S: {reply}");
                    let args: Vec<&str> = reply.trim().split(' ').collect();

                    if *args.first().unwrap_or(&"") == "QNG" {
                        // Parse and sanity check to avoid spamming the server
                        if let Ok(duration) = args.get(1).unwrap_or(&"").parse()
                            && duration > 5
                        {
                            tokio::time::sleep(Duration::from_secs(duration)).await;
                            break;
                        } else {
                            break 'outer;
                        }
                    }
                }
            }

            if let Err(error) = event_tx.send(Event::Disconnected).await {
                error!("{error}");
            }
        });
    }

    async fn handle_switchboard_invitations(&self) -> Result<(), SdkError> {
        let event_tx = self.event_tx.clone();
        let mut internal_rx = self.internal_tx.subscribe();

        let user_data = self.user_data.clone();
        let user_email;
        {
            let user_data = self
                .user_data
                .read()
                .or(Err(SdkError::CouldNotGetUserData))?;

            user_email = user_data
                .email
                .as_ref()
                .ok_or(SdkError::NotLoggedIn)?
                .clone();
        }

        tokio::spawn(async move {
            while let Ok(event) = internal_rx.recv().await {
                if let InternalEvent::SwitchboardInvitation {
                    server,
                    port,
                    session_id,
                    cki_string,
                } = event
                {
                    let switchboard = Switchboard::new(
                        server.as_str(),
                        port.as_str(),
                        cki_string.as_str(),
                        user_data.clone(),
                    )
                    .await;

                    if let Ok(switchboard) = switchboard {
                        if switchboard.answer(&user_email, &session_id).await.is_ok() {
                            if let Err(error) = event_tx
                                .send(Event::SessionAnswered(Arc::new(switchboard)))
                                .await
                            {
                                error!("{error}");
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Adds a handler closure. If you're using this SDK with Rust, not through a foreign language binding, then this is the preferred
    /// method of receiving and handling events.
    pub fn add_event_handler_closure<F, R>(&self, f: F)
    where
        F: Fn(Event) -> R + Send + 'static,
        R: Future<Output = ()> + Send,
    {
        let event_rx = self.event_rx.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                f(event).await;
            }
        });
    }

    /// Adds a new handler that implements the [EventHandler] trait.
    ///
    /// This exists for the foreign language bindings, with which generics don't
    /// work. Prefer [`add_event_handler_closure`][Client::add_event_handler_closure] if using this SDK with Rust.
    pub fn add_event_handler(&self, handler: Arc<dyn EventHandler>) {
        let event_rx = self.event_rx.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                handler.handle(event).await;
            }
        });
    }

    /// Does the MSNP authentication process. Also starts regular pings and the handler for Switchboard invitations.
    ///
    /// # Events
    /// If the server you're connecting to implements a Dispatch Server, then this will return a [RedirectedTo][Event::RedirectedTo] event.
    /// What follows is [creating a new][Client::new] client instance with the server and port returned then logging in again, which
    /// will return an [Authenticated][Event::Authenticated] event.
    pub async fn login(
        &self,
        email: String,
        password: &str,
        nexus_url: &str,
        client_name: &str,
        version: &str,
    ) -> Result<Event, SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();

        ver::send(&self.tr_id, &self.ns_tx, &mut internal_rx).await?;
        cvr::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            &email,
            client_name,
            version,
        )
        .await?;

        let authorization_string =
            match usr_i::send(&self.tr_id, &self.ns_tx, &mut internal_rx, &email).await? {
                InternalEvent::GotAuthorizationString(authorization_string) => authorization_string,
                InternalEvent::RedirectedTo { server, port } => {
                    return Ok(Event::RedirectedTo { server, port });
                }

                _ => return Err(SdkError::CouldNotGetAuthenticationString),
            };

        let auth = PassportAuth::new(nexus_url);
        let token = auth
            .get_passport_token(&email, password, &authorization_string)
            .await?;

        usr_s::send(&self.tr_id, &self.ns_tx, &mut internal_rx, &token).await?;

        {
            let mut user_data = self
                .user_data
                .write()
                .or(Err(SdkError::CouldNotSetUserData))?;

            user_data.email = Some(email);
        }

        syn::send(&self.tr_id, &self.ns_tx, &mut internal_rx).await?;
        gcf::send(&self.tr_id, &self.ns_tx, &mut internal_rx).await?;

        self.handle_switchboard_invitations().await?;
        self.start_pinging();

        Ok(Event::Authenticated)
    }

    /// Sets the user's presence status.
    pub async fn set_presence(&self, presence: MsnpStatus) -> Result<(), SdkError> {
        let msn_object;
        {
            let user_data = self
                .user_data
                .read()
                .or(Err(SdkError::CouldNotSetUserData))?;

            msn_object = user_data.msn_object.clone();
        }
        let mut internal_rx = self.internal_tx.subscribe();

        let presence = Presence::new(
            presence,
            if let Some(msn_object) = &msn_object {
                quick_xml::de::from_str(msn_object).ok()
            } else {
                None
            },
            msn_object,
        );

        chg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, &presence).await
    }

    /// Sets the user's personal message.
    pub async fn set_personal_message(
        &self,
        personal_message: &PersonalMessage,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        uux::send(&self.tr_id, &self.ns_tx, &mut internal_rx, personal_message).await
    }

    /// Sets the user's display name.
    pub async fn set_display_name(&self, display_name: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        prp::send(&self.tr_id, &self.ns_tx, &mut internal_rx, display_name).await
    }

    /// Sets a contact's display name.
    pub async fn set_contact_display_name(
        &self,
        guid: &str,
        display_name: &str,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        sbp::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            guid,
            display_name,
        )
        .await
    }

    /// Adds a contact to a specified list, also setting its display name if applicable.
    pub async fn add_contact(
        &self,
        email: &str,
        display_name: &str,
        list: MsnpList,
    ) -> Result<Event, SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        adc::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            display_name,
            list,
        )
        .await
    }

    /// Removes a contact from a specified list (except the forward list, which requires calling
    /// [remove_contact_from_forward_list][Client::remove_contact_from_forward_list]).
    pub async fn remove_contact(&self, email: &str, list: MsnpList) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        rem::send(&self.tr_id, &self.ns_tx, &mut internal_rx, email, list).await
    }

    /// Removes a contact from the forward list.
    pub async fn remove_contact_from_forward_list(&self, guid: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        rem::send_with_forward_list(&self.tr_id, &self.ns_tx, &mut internal_rx, guid).await
    }

    /// Blocks a contact.
    pub async fn block_contact(&self, email: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        adc::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            email,
            MsnpList::BlockList,
        )
        .await?;

        rem::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            MsnpList::AllowList,
        )
        .await
    }

    /// Unblocks a contact.
    pub async fn unblock_contact(&self, email: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        adc::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            email,
            MsnpList::AllowList,
        )
        .await?;

        rem::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            MsnpList::BlockList,
        )
        .await
    }

    /// Creates a new contact group.
    pub async fn create_group(&self, name: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        adg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, name).await
    }

    /// Deletes a contact group.
    pub async fn delete_group(&self, guid: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        rmg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, guid).await
    }

    /// Renames a contact group.
    pub async fn rename_group(&self, guid: &str, new_name: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        reg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, guid, new_name).await
    }

    /// Adds a contact to a group.
    pub async fn add_contact_to_group(&self, guid: &str, group_guid: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        adc::send_with_group(&self.tr_id, &self.ns_tx, &mut internal_rx, guid, group_guid).await
    }

    /// Removes a contact from a group.
    pub async fn remove_contact_from_group(
        &self,
        guid: &str,
        group_guid: &str,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        rem::send_with_group(&self.tr_id, &self.ns_tx, &mut internal_rx, guid, group_guid).await
    }

    /// Sets the GTC value, which can be either `A` or `N`.
    pub async fn set_gtc(&self, gtc: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        gtc::send(&self.tr_id, &self.ns_tx, &mut internal_rx, gtc).await
    }

    /// Sets the GTC value, which can be either `AL` or `BL`.
    pub async fn set_blp(&self, blp: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        blp::send(&self.tr_id, &self.ns_tx, &mut internal_rx, blp).await
    }

    /// Creates and returns a new Switchboard session with the specified contact.
    pub async fn create_session(&self, email: &str) -> Result<Switchboard, SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        let user_email;
        {
            let user_data = self
                .user_data
                .read()
                .or(Err(SdkError::CouldNotGetUserData))?;

            user_email = user_data
                .email
                .as_ref()
                .ok_or(SdkError::NotLoggedIn)?
                .clone();
        }

        let switchboard = xfr::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            self.user_data.clone(),
        )
        .await?;

        switchboard.login(&user_email).await?;
        switchboard.invite(email).await?;
        Ok(switchboard)
    }

    /// Sets the user's display picture, returning a standard base64 encoded hash of it.
    /// This method uses the picture's binary data, and scaling down beforehand to a size like 200x200 is recommended.
    pub fn set_display_picture(&self, display_picture: Vec<u8>) -> Result<String, SdkError> {
        let mut user_data = self
            .user_data
            .write()
            .or(Err(SdkError::CouldNotSetUserData))?;

        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

        let mut hash = sha1_smol::Sha1::new();
        hash.update(display_picture.as_slice());
        let sha1d = STANDARD.encode(hash.digest().bytes());

        let sha1c = format!(
            "Creator{user_email}Size{}Type3LocationPIC.tmpFriendlyAAA=SHA1D{sha1d}",
            display_picture.len()
        );

        let mut hash = sha1_smol::Sha1::new();
        hash.update(sha1c.as_bytes());
        let sha1c = STANDARD.encode(hash.digest().bytes());

        user_data.msn_object = Some(format!(
            "<msnobj Creator=\"{user_email}\" Size=\"{}\" Type=\"3\" Location=\"PIC.tmp\" Friendly=\"AAA=\" SHA1D=\"{sha1d}\" SHA1C=\"{sha1c}\"/>",
            display_picture.len()
        ));

        user_data.display_picture = Some(display_picture);
        Ok(sha1d)
    }

    /// Disconnects from the server.
    pub async fn disconnect(&self) -> Result<(), SdkError> {
        let command = "OUT\r\n";
        trace!("C: {command}");

        self.ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        self.event_tx.close();
        Ok(())
    }
}
