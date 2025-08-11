use crate::errors::p2p_error::P2pError;
use crate::errors::sdk_error::SdkError;
use crate::internal_event::InternalEvent;
use crate::switchboard_server::commands::msg;
use crate::switchboard_server::p2p::display_picture_session::DisplayPictureSession;
use crate::user_data::UserData;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use tokio::sync::RwLock;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;

pub async fn handle_invite(
    destination: String,
    invite: Vec<u8>,
    user_data: Arc<RwLock<UserData>>,
    command_internal_rx: &mut Receiver<InternalEvent>,
    tr_id: Arc<AtomicU32>,
    sb_tx: Sender<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let user_data = user_data.read().await;
        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

        if destination != *user_email {
            return Err(P2pError::OtherDestination.into());
        }
    }

    let invite_string = unsafe { str::from_utf8_unchecked(invite.as_slice()) };
    let mut invite_parameters = invite_string.lines();

    invite_parameters.next();
    let to = invite_parameters
        .next()
        .ok_or(P2pError::CouldNotGetSessionData)?;

    {
        let user_data = user_data.read().await;
        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

        if !to.contains(format!("msnmsgr:{user_email}").as_str()) {
            return Err(P2pError::OtherDestination.into());
        }
    }

    let from = invite_parameters
        .next()
        .ok_or(P2pError::CouldNotGetSessionData)?
        .replace("From: <msnmsgr:", "")
        .replace(">", "");

    let session = DisplayPictureSession::new_from_invite(&invite)?;
    let ack_payload = DisplayPictureSession::acknowledge(&invite)?;

    msg::send_p2p(
        &tr_id,
        &sb_tx,
        command_internal_rx,
        ack_payload,
        from.as_str(),
    )
    .await?;

    let context = invite_parameters
        .find(|line| line.contains("Context: "))
        .ok_or(P2pError::CouldNotGetSessionData)?
        .replace("Context: ", "");

    {
        let user_data = user_data.read().await;
        let msn_object = user_data
            .msn_object
            .as_ref()
            .ok_or(SdkError::CouldNotGetUserData)?;

        if context != STANDARD.encode((msn_object.to_owned() + "\0").as_bytes()) {
            return Err(P2pError::OtherContext.into());
        }
    }

    let ok_payload = session.ok(from.as_str(), to)?;
    msg::send_p2p(
        &tr_id,
        &sb_tx,
        command_internal_rx,
        ok_payload,
        from.as_str(),
    )
    .await?;

    let preparation_payload = session.data_preparation()?;
    msg::send_p2p(
        &tr_id,
        &sb_tx,
        command_internal_rx,
        preparation_payload,
        from.as_str(),
    )
    .await?;

    let user_data = user_data.read().await;
    let display_picture = user_data
        .display_picture
        .as_ref()
        .ok_or(SdkError::CouldNotGetDisplayPicture)?;

    let data_payloads = session.data(display_picture)?;
    for data_payload in data_payloads {
        msg::send_p2p(
            &tr_id,
            &sb_tx,
            command_internal_rx,
            data_payload,
            from.as_str(),
        )
        .await?;
    }

    Ok(())
}

pub async fn handle_bye(
    destination: String,
    bye: Vec<u8>,
    user_data: Arc<RwLock<UserData>>,
    command_internal_rx: &mut Receiver<InternalEvent>,
    tr_id: Arc<AtomicU32>,
    sb_tx: Sender<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let user_data = user_data.read().await;
        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

        if destination != *user_email {
            return Err(P2pError::OtherDestination.into());
        }
    }

    let bye_string = unsafe { str::from_utf8_unchecked(bye.as_slice()) };
    let mut bye_parameters = bye_string.lines();

    let from = bye_parameters
        .nth(2)
        .ok_or(P2pError::CouldNotGetSessionData)?
        .replace("From: <msnmsgr:", "")
        .replace(">", "");

    let ack_payload = DisplayPictureSession::acknowledge(&bye)?;

    msg::send_p2p(
        &tr_id,
        &sb_tx,
        command_internal_rx,
        ack_payload,
        from.as_str(),
    )
    .await
    .map_err(|error| error.into())
}
