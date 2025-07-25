#[tokio::test]
async fn create_session() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let mut client = msnp11_sdk::client::Client::new("127.0.0.1", 1863)
        .await
        .unwrap();

    let result: msnp11_sdk::enums::event::Event = match client
        .login(
            "testing@example.com".to_string(),
            "123456",
            "http://localhost:3000/rdr/pprdr.asp",
            "msnp11-sdk",
            "0.6",
        )
        .await
    {
        Ok(msnp11_sdk::enums::event::Event::RedirectedTo { server, port }) => {
            client = msnp11_sdk::client::Client::new(&*server, port)
                .await
                .unwrap();
            client
                .login(
                    "testing@example.com".to_string(),
                    "123456",
                    "http://localhost:3000/rdr/pprdr.asp",
                    "msnp11-sdk",
                    "0.6",
                )
                .await
                .unwrap()
        }

        Ok(msnp11_sdk::enums::event::Event::Authenticated) => {
            msnp11_sdk::enums::event::Event::Authenticated
        }
        Err(err) => panic!("Login error: {err}"),

        _ => msnp11_sdk::enums::event::Event::Disconnected,
    };

    assert!(matches!(
        result,
        msnp11_sdk::enums::event::Event::Authenticated
    ));

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

    switchboard.add_event_handler_closure(|event| async {
        match event {
            msnp11_sdk::enums::event::Event::ParticipantInSwitchboard { email } => {
                assert_eq!(email, "bob@passport.com");
            }

            msnp11_sdk::enums::event::Event::TextMessage { email, message } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(message.color, "ff0000");
                assert_eq!(message.text, "h");
            }

            msnp11_sdk::enums::event::Event::Nudge { email } => {
                assert_eq!(email, "bob@passport.com");
            }

            msnp11_sdk::enums::event::Event::ParticipantLeftSwitchboard { email } => {
                assert_eq!(email, "bob@passport.com");
            }

            _ => (),
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    switchboard.disconnect().await.unwrap();
    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn join_session() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let mut client = msnp11_sdk::client::Client::new("127.0.0.1", 1863)
        .await
        .unwrap();

    let result: msnp11_sdk::enums::event::Event = match client
        .login(
            "testing@example.com".to_string(),
            "123456",
            "http://localhost:3000/rdr/pprdr.asp",
            "msnp11-sdk",
            "0.6",
        )
        .await
    {
        Ok(msnp11_sdk::enums::event::Event::RedirectedTo { server, port }) => {
            client = msnp11_sdk::client::Client::new(&*server, port)
                .await
                .unwrap();
            client
                .login(
                    "testing@example.com".to_string(),
                    "123456",
                    "http://localhost:3000/rdr/pprdr.asp",
                    "msnp11-sdk",
                    "0.6",
                )
                .await
                .unwrap()
        }

        Ok(msnp11_sdk::enums::event::Event::Authenticated) => {
            msnp11_sdk::enums::event::Event::Authenticated
        }
        Err(err) => panic!("Login error: {err}"),
        _ => msnp11_sdk::enums::event::Event::Disconnected,
    };

    assert!(matches!(
        result,
        msnp11_sdk::enums::event::Event::Authenticated
    ));

    // GTC abuse from the mock server
    client.set_gtc(&"ReceiveRNG".to_string()).await.unwrap();

    client.add_event_handler_closure(|event| async {
        match event {
            msnp11_sdk::enums::event::Event::SessionAnswered(switchboard) => {
                switchboard.add_event_handler_closure(|event| async {
                    match event {
                        msnp11_sdk::enums::event::Event::ParticipantInSwitchboard { email } => {
                            assert_eq!(email, "bob@passport.com");
                        }

                        msnp11_sdk::enums::event::Event::TextMessage { email, message } => {
                            assert_eq!(email, "bob@passport.com");
                            assert_eq!(message.color, "ff0000");
                            assert_eq!(message.text, "h");
                        }

                        msnp11_sdk::enums::event::Event::Nudge { email } => {
                            assert_eq!(email, "bob@passport.com");
                        }

                        msnp11_sdk::enums::event::Event::ParticipantLeftSwitchboard { email } => {
                            assert_eq!(email, "bob@passport.com");
                        }

                        _ => (),
                    }
                });
            }

            _ => (),
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    client.disconnect().await.unwrap();
}
