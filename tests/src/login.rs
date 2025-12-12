#[tokio::test]
async fn login() {
    let mut client = msnp11_sdk::client::Client::new(&"127.0.0.1".to_string(), 1863)
        .await
        .unwrap();

    if let Ok(msnp11_sdk::enums::event::Event::RedirectedTo { server, port }) = client
        .login(
            "testing@example.com".to_string(),
            "123456",
            "http://localhost:3000/rdr/pprdr.asp",
            "msnp11-sdk",
            "0.6",
        )
        .await
    {
        client = msnp11_sdk::client::Client::new(&server, port)
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
            .unwrap();
    }

    client
        .set_presence(msnp11_sdk::enums::msnp_status::MsnpStatus::Online)
        .await
        .unwrap();

    client
        .set_personal_message(&msnp11_sdk::models::personal_message::PersonalMessage {
            psm: "test".to_string(),
            current_media: "".to_string(),
        })
        .await
        .unwrap();

    client.add_event_handler_closure(|event| async {
        match event {
            msnp11_sdk::enums::event::Event::Gtc(gtc) => assert_eq!(gtc, "A"),
            msnp11_sdk::enums::event::Event::Blp(blp) => assert_eq!(blp, "AL"),
            msnp11_sdk::enums::event::Event::DisplayName(display_name) => {
                assert_eq!(display_name, "Testing")
            }

            msnp11_sdk::enums::event::Event::Group { name, guid: id } => {
                assert_eq!(name, "Mock Contacts");
                assert_eq!(id, "124153dc-a695-4f6c-93e8-8e07c9775251");
            }

            msnp11_sdk::enums::event::Event::ContactInForwardList {
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
                        msnp11_sdk::enums::msnp_list::MsnpList::ForwardList,
                        msnp11_sdk::enums::msnp_list::MsnpList::BlockList,
                        msnp11_sdk::enums::msnp_list::MsnpList::ReverseList
                    ]
                )
            }

            msnp11_sdk::enums::event::Event::Contact {
                email,
                display_name,
                lists,
            } => {
                assert_eq!(email, "fred@passport.com");
                assert_eq!(display_name, "Fred");
                assert_eq!(
                    lists,
                    vec![msnp11_sdk::enums::msnp_list::MsnpList::AllowList]
                )
            }

            msnp11_sdk::enums::event::Event::PresenceUpdate {
                email,
                display_name,
                presence,
            } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(display_name, "Bob");
                assert_eq!(presence.msn_object.as_ref().unwrap().creator, email);
                assert_eq!(presence.msn_object.as_ref().unwrap().size, 22731);
                assert_eq!(presence.msn_object.as_ref().unwrap().object_type, 3);
            }

            msnp11_sdk::enums::event::Event::PersonalMessageUpdate {
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

            msnp11_sdk::enums::event::Event::ServerMaintenance { time_remaining } => {
                assert_eq!(time_remaining, 5);
            }

            _ => (),
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    client.disconnect().await.unwrap();
}
