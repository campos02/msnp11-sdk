
#[tokio::test]
async fn messaging() {
    let mut client = msnp11_sdk::client::Client::new("127.0.0.1".to_string(), "1863".to_string())
        .await
        .unwrap();

    let result: msnp11_sdk::event::Event = match client
        .login(
            "testing@example.com".to_string(),
            "123456".to_string(),
            "http://localhost:3000/rdr/pprdr.asp".to_string(),
        )
        .await
    {
        Ok(msnp11_sdk::event::Event::RedirectedTo { server, port }) => {
            client = msnp11_sdk::client::Client::new(server, port).await.unwrap();
            client
                .login(
                    "testing@example.com".to_string(),
                    "123456".to_string(),
                    "http://localhost:3000/rdr/pprdr.asp".to_string(),
                )
                .await
                .unwrap()
        }

        Ok(msnp11_sdk::event::Event::Authenticated) => msnp11_sdk::event::Event::Authenticated,
        Err(err) => panic!("Login error: {err}"),
        _ => msnp11_sdk::event::Event::Disconnected,
    };

    assert_eq!(result, msnp11_sdk::event::Event::Authenticated);

    let message = msnp11_sdk::models::plain_text::PlainText {
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
        color: "ff".to_string(),
        text: "h".to_string(),
    };

    client
        .send_text_message(&message, &"bob@passport.com".to_string())
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    for _ in 0..client.event_queue_size() {
        match client.receive_event().await.unwrap() {
            msnp11_sdk::event::Event::ParticipantInSwitchboard { session_id, email } => {
                assert_eq!(session_id, "11752013");
                assert_eq!(email, "bob@passport.com");
            }

            msnp11_sdk::event::Event::TextMessage {
                session_id,
                email,
                message,
            } => {
                assert_eq!(session_id, "11752013");
                assert_eq!(email, "bob@passport.com");
                assert_eq!(message.color, "ff0000");
                assert_eq!(message.text, "h");
            }

            msnp11_sdk::event::Event::Nudge { session_id, email } => {
                assert_eq!(session_id, "11752013");
                assert_eq!(email, "bob@passport.com");
            }

            msnp11_sdk::event::Event::ParticipantLeftSwitchboard { session_id, email } => {
                assert_eq!(session_id, "11752013");
                assert_eq!(email, "bob@passport.com");
            }

            _ => (),
        }
    }

    client
        .send_text_message(&message, &"bob@passport.com".to_string())
        .await
        .unwrap();

    client.disconnect().await.unwrap();
}
