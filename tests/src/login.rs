#[tokio::test]
async fn login() {
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

    client.set_presence("NLN".to_string()).await.unwrap();
    client
        .set_personal_message(&msnp11_sdk::models::personal_message::PersonalMessage {
            psm: "test".to_string(),
            current_media: "".to_string(),
        })
        .await
        .unwrap();

    assert!(client.event_queue_size() >= 8);
    for _ in 0..client.event_queue_size() {
        match client.receive_event().await.unwrap() {
            msnp11_sdk::event::Event::Gtc(gtc) => assert_eq!(gtc, "A"),
            msnp11_sdk::event::Event::Blp(blp) => assert_eq!(blp, "AL"),
            msnp11_sdk::event::Event::DisplayName(display_name) => {
                assert_eq!(display_name, "Testing")
            }

            msnp11_sdk::event::Event::Group { name, guid: id } => {
                assert_eq!(name, "Mock Contacts");
                assert_eq!(id, "124153dc-a695-4f6c-93e8-8e07c9775251");
            }

            msnp11_sdk::event::Event::ContactInForwardList {
                email,
                display_name,
                guid,
                lists,
                groups,
            } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(display_name, "Bob");
                assert_eq!(guid, "6bd736b8-dc18-44c6-ad61-8cd12d641e79");
                assert_eq!(groups, vec!["124153dc-a695-4f6c-93e8-8e07c9775251"]);
                assert_eq!(
                    lists,
                    vec![
                        msnp11_sdk::list::List::ForwardList,
                        msnp11_sdk::list::List::BlockList,
                        msnp11_sdk::list::List::ReverseList
                    ]
                )
            }

            msnp11_sdk::event::Event::Contact {
                email,
                display_name,
                lists,
            } => {
                assert_eq!(email, "fred@passport.com");
                assert_eq!(display_name, "Fred");
                assert_eq!(lists, vec![msnp11_sdk::list::List::AllowList])
            }

            msnp11_sdk::event::Event::PresenceUpdate {
                email,
                display_name,
                presence,
            } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(display_name, "Bob");
                assert_eq!(
                    presence,
                    msnp11_sdk::models::presence::Presence {
                        presence: "NLN".to_string(),
                        client_id: 1073741824,
                        msn_object: Some("<msnobj Creator=\"".to_string())
                    }
                );
            }

            msnp11_sdk::event::Event::PersonalMessageUpdate {
                email,
                personal_message,
            } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(
                    personal_message,
                    msnp11_sdk::models::personal_message::PersonalMessage {
                        psm: "my msn all ducked".to_string(),
                        current_media: "".to_string()
                    }
                );
            }

            _ => (),
        }
    }

    client.disconnect().await.unwrap();
}
