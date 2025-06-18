use crate::event::Event;
use crate::event_handler::EventHandler;
use crate::internal_event::InternalEvent;
use crate::models::personal_message::PersonalMessage;
use crate::models::presence::Presence;
use crate::models::user_data::UserData;
use crate::msnp_list::MsnpList;
use crate::notification_server::commands::adc::Adc;
use crate::notification_server::commands::adg::Adg;
use crate::notification_server::commands::blp::Blp;
use crate::notification_server::commands::chg::Chg;
use crate::notification_server::commands::cvr::Cvr;
use crate::notification_server::commands::gcf::Gcf;
use crate::notification_server::commands::gtc::Gtc;
use crate::notification_server::commands::prp::Prp;
use crate::notification_server::commands::reg::Reg;
use crate::notification_server::commands::rem::Rem;
use crate::notification_server::commands::rmg::Rmg;
use crate::notification_server::commands::sbp::Sbp;
use crate::notification_server::commands::syn::Syn;
use crate::notification_server::commands::usr_i::UsrI;
use crate::notification_server::commands::usr_s::UsrS;
use crate::notification_server::commands::uux::Uux;
use crate::notification_server::commands::ver::Ver;
use crate::notification_server::commands::xfr::Xfr;
use crate::notification_server::event_matcher::{into_event, into_internal_event};
use crate::passport_auth::PassportAuth;
use crate::receive_split_into_base64::receive_split_into_base64;
use crate::sdk_error::SdkError;
use crate::switchboard::switchboard::Switchboard;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use log::trace;
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
    pub async fn new(server: String, port: String) -> Result<Self, SdkError> {
        let server_ip = lookup_host(format!("{server}:{port}"))
            .await
            .or(Err(SdkError::ResolutionError))?
            .next()
            .ok_or(SdkError::ResolutionError)?
            .ip()
            .to_string();

        let (event_tx, event_rx) = async_channel::bounded::<Event>(32);
        let (ns_tx, mut ns_rx) = mpsc::channel::<Vec<u8>>(16);
        let (internal_tx, _) = broadcast::channel::<InternalEvent>(64);

        let socket = TcpStream::connect(format!("{server_ip}:{port}"))
            .await
            .or(Err(SdkError::CouldNotConnectToServer))?;

        let (mut rd, mut wr) = socket.into_split();

        let internal_task_tx = internal_tx.clone();
        let event_task_tx = event_tx.clone();
        tokio::spawn(async move {
            while let Ok(base64_messages) = receive_split_into_base64(&mut rd).await {
                for base64_message in base64_messages {
                    let internal_event = into_internal_event(&base64_message);
                    internal_task_tx
                        .send(internal_event)
                        .expect("Error sending internal event to channel");

                    let event = into_event(&base64_message);
                    if let Some(event) = event {
                        event_task_tx
                            .send(event)
                            .await
                            .expect("Error sending event to channel");
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Some(message) = ns_rx.recv().await {
                wr.write_all(message.as_slice())
                    .await
                    .expect("Error sending message to socket");
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
            while ns_tx.send(command.as_bytes().to_vec()).await.is_ok() {
                trace!("C: {command}");

                while let Ok(InternalEvent::ServerReply(reply)) = internal_rx.recv().await {
                    trace!("S: {reply}");

                    let args: Vec<&str> = reply.trim().split(' ').collect();
                    match args[0] {
                        "QNG" => {
                            let duration = args[1].parse().unwrap_or(50);
                            tokio::time::sleep(Duration::from_secs(duration)).await;
                            break;
                        }

                        _ => (),
                    }
                }
            }

            event_tx
                .send(Event::Disconnected)
                .await
                .expect("Error sending disconnection event");
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
                match event {
                    InternalEvent::SwitchboardInvitation {
                        server,
                        port,
                        session_id,
                        cki_string,
                    } => {
                        let switchboard = Switchboard::new(
                            server.as_str(),
                            port.as_str(),
                            cki_string.as_str(),
                            user_data.clone(),
                        )
                        .await;

                        if let Ok(switchboard) = switchboard {
                            if switchboard.answer(&user_email, &session_id).await.is_ok() {
                                event_tx
                                    .send(Event::SessionAnswered(Arc::new(switchboard)))
                                    .await
                                    .expect("Could not send invitation event to channel");
                            }
                        }
                    }

                    _ => (),
                }
            }
        });

        Ok(())
    }

    /// Adds a handler closure. If you're using this SDK with Rust, not through a foreign binding, then this is the preferred method of
    /// handling events.
    pub fn add_event_handler_closure<F>(&self, f: F)
    where
        F: Fn(Event) + Send + 'static,
    {
        let event_rx = self.event_rx.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                f(event);
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

    /// Does the MSNP authentication process. Also starts regular pings and the handler for switchboard invitations.
    ///
    /// # Events
    /// If the server you're connecting to implements a Dispatch Server, then this will return a [RedirectedTo][Event::RedirectedTo] event.
    /// The proceeding is to [create a new][Client::new] instance with the server and port returned as arguments, then login normally, which
    /// will return an [Authenticated][Event::Authenticated] event.
    pub async fn login(
        &self,
        email: String,
        password: String,
        nexus_url: String,
    ) -> Result<Event, SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();

        Ver::send(&self.tr_id, &self.ns_tx, &mut internal_rx).await?;
        Cvr::send(&self.tr_id, &self.ns_tx, &mut internal_rx, &email).await?;

        let authorization_string =
            match UsrI::send(&self.tr_id, &self.ns_tx, &mut internal_rx, &email).await? {
                InternalEvent::GotAuthorizationString(authorization_string) => authorization_string,
                InternalEvent::RedirectedTo { server, port } => {
                    return Ok(Event::RedirectedTo { server, port });
                }

                _ => return Err(SdkError::CouldNotGetAuthenticationString.into()),
            };

        let auth = PassportAuth::new(nexus_url);
        let token = auth
            .get_passport_token(&email, password, authorization_string)
            .await?;

        UsrS::send(&self.tr_id, &self.ns_tx, &mut internal_rx, &token).await?;

        {
            let mut user_data = self
                .user_data
                .write()
                .or(Err(SdkError::CouldNotSetUserData))?;

            user_data.email = Some(email);
        }

        Syn::send(&self.tr_id, &self.ns_tx, &mut internal_rx).await?;
        Gcf::send(&self.tr_id, &self.ns_tx, &mut internal_rx).await?;

        self.handle_switchboard_invitations().await?;
        self.start_pinging();

        Ok(Event::Authenticated)
    }

    /// Sets the user's presence status. One of MSNP's status strings(e.g. `NLN` for online) is required as the argument.
    pub async fn set_presence(&self, presence: String) -> Result<(), SdkError> {
        let msn_object;
        {
            let user_data = self
                .user_data
                .read()
                .or(Err(SdkError::CouldNotSetUserData))?;

            msn_object = user_data.msn_object.clone();
        }
        let mut internal_rx = self.internal_tx.subscribe();

        let presence = Presence::new(presence, msn_object);
        Chg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, &presence).await
    }

    /// Sets the user's personal message.
    pub async fn set_personal_message(
        &self,
        personal_message: &PersonalMessage,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Uux::send(&self.tr_id, &self.ns_tx, &mut internal_rx, personal_message).await
    }

    /// Sets the user's display name.
    pub async fn set_display_name(&self, display_name: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Prp::send(&self.tr_id, &self.ns_tx, &mut internal_rx, display_name).await
    }

    /// Sets a contact's display name.
    pub async fn set_contact_display_name(
        &self,
        guid: &String,
        display_name: &String,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Sbp::send(
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
        email: &String,
        display_name: &String,
        list: MsnpList,
    ) -> Result<Event, SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Adc::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            display_name,
            list,
        )
        .await
    }

    /// Removes a contact from a specified list that's not the forward list, that requires a GUID and calling
    /// [remove_contact_from_forward_list][Client::remove_contact_from_forward_list].
    pub async fn remove_contact(&self, email: &String, list: MsnpList) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Rem::send(&self.tr_id, &self.ns_tx, &mut internal_rx, email, list).await
    }

    /// Removes a contact from the forward list.
    pub async fn remove_contact_from_forward_list(&self, guid: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Rem::send_with_forward_list(&self.tr_id, &self.ns_tx, &mut internal_rx, guid).await
    }

    /// Blocks a contact.
    pub async fn block_contact(&self, email: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Adc::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            email,
            MsnpList::BlockList,
        )
        .await?;

        Rem::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            MsnpList::AllowList,
        )
        .await
    }

    /// Unblocks a contact.
    pub async fn unblock_contact(&self, email: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Adc::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            email,
            MsnpList::AllowList,
        )
        .await?;

        Rem::send(
            &self.tr_id,
            &self.ns_tx,
            &mut internal_rx,
            email,
            MsnpList::BlockList,
        )
        .await
    }

    /// Creates a new contact group.
    pub async fn create_group(&self, name: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Adg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, name).await
    }

    /// Deletes a contact group.
    pub async fn delete_group(&self, guid: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Rmg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, guid).await
    }

    /// Renames a contact group.
    pub async fn rename_group(&self, guid: &String, new_name: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Reg::send(&self.tr_id, &self.ns_tx, &mut internal_rx, guid, new_name).await
    }

    /// Adds a contact to a group.
    pub async fn add_contact_to_group(
        &self,
        guid: &String,
        group_guid: &String,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Adc::send_with_group(&self.tr_id, &self.ns_tx, &mut internal_rx, guid, group_guid).await
    }

    /// Removes a contact from a group.
    pub async fn remove_contact_from_group(
        &self,
        guid: &String,
        group_guid: &String,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Rem::send_with_group(&self.tr_id, &self.ns_tx, &mut internal_rx, guid, group_guid).await
    }

    /// Sets the GTC value, which can be either `A` or `N`.
    pub async fn set_gtc(&self, gtc: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Gtc::send(&self.tr_id, &self.ns_tx, &mut internal_rx, gtc).await
    }

    /// Sets the GTC value, which can be either `AL` or `BL`.
    pub async fn set_blp(&self, blp: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Blp::send(&self.tr_id, &self.ns_tx, &mut internal_rx, blp).await
    }

    /// Creates a new switchboard with the specified contact. Returns the created SB, which is used for messaging.
    pub async fn create_session(&self, email: &String) -> Result<Switchboard, SdkError> {
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

        let switchboard = Xfr::send(
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

    /// Sets the user's display picture. This method uses the picture's binary data, and scaling down to a size like 96x96 is recommended.
    pub fn set_display_picture(&self, display_picture: Vec<u8>) -> Result<(), SdkError> {
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
        Ok(())
    }

    /// Disconnects from the server.
    pub async fn disconnect(&self) -> Result<(), SdkError> {
        let command = "OUT\r\n";
        trace!("C: {command}");
        self.ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))
    }
}
