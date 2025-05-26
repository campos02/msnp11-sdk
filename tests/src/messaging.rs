#[tokio::test]
async fn create_session() {
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

    assert!(matches!(result, msnp11_sdk::event::Event::Authenticated));

    let message = msnp11_sdk::models::plain_text::PlainText {
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
        color: "ff".to_string(),
        text: "h".to_string(),
    };

    let switchboard = client
        .create_session(&"bob@passport.com".to_string())
        .await
        .unwrap();

    switchboard.send_text_message(&message).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(switchboard.event_queue_size() >= 4);

    for _ in 0..switchboard.event_queue_size() {
        match switchboard.receive_event().await.unwrap() {
            msnp11_sdk::event::Event::ParticipantInSwitchboard { email } => {
                assert_eq!(email, "bob@passport.com");
            }

            msnp11_sdk::event::Event::TextMessage { email, message } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(message.color, "ff0000");
                assert_eq!(message.text, "h");
            }

            msnp11_sdk::event::Event::Nudge { email } => {
                assert_eq!(email, "bob@passport.com");
            }

            msnp11_sdk::event::Event::ParticipantLeftSwitchboard { email } => {
                assert_eq!(email, "bob@passport.com");
            }

            _ => (),
        }
    }

    switchboard.disconnect().await.unwrap();
    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn join_session() {
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

    assert!(matches!(result, msnp11_sdk::event::Event::Authenticated));

    // GTC abuse from the mock server
    client.set_gtc(&"ReceiveRNG".to_string()).await.unwrap();
    loop {
        match client.receive_event().await.unwrap() {
            msnp11_sdk::event::Event::SessionAnswered(switchboard) => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                assert!(switchboard.event_queue_size() >= 4);

                for _ in 0..switchboard.event_queue_size() {
                    match switchboard.receive_event().await.unwrap() {
                        msnp11_sdk::event::Event::ParticipantInSwitchboard { email } => {
                            assert_eq!(email, "bob@passport.com");
                        }

                        msnp11_sdk::event::Event::TextMessage { email, message } => {
                            assert_eq!(email, "bob@passport.com");
                            assert_eq!(message.color, "ff0000");
                            assert_eq!(message.text, "h");
                        }

                        msnp11_sdk::event::Event::Nudge { email } => {
                            assert_eq!(email, "bob@passport.com");
                        }

                        msnp11_sdk::event::Event::ParticipantLeftSwitchboard { email } => {
                            assert_eq!(email, "bob@passport.com");
                        }

                        _ => (),
                    }
                }

                switchboard.disconnect().await.unwrap();
                break;
            }

            _ => (),
        }
    }

    client.disconnect().await.unwrap();
}
