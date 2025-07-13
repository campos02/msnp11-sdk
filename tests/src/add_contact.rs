#[tokio::test]
async fn add_contact() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let mut client = msnp11_sdk::client::Client::new("127.0.0.1", &1863)
        .await
        .unwrap();

    let result: msnp11_sdk::enums::event::Event = match client
        .login(
            "testing@example.com".to_string(),
            "123456",
            "http://localhost:3000/rdr/pprdr.asp",
        )
        .await
    {
        Ok(msnp11_sdk::enums::event::Event::RedirectedTo { server, port }) => {
            client = msnp11_sdk::client::Client::new(&*server, &port)
                .await
                .unwrap();
            client
                .login(
                    "testing@example.com".to_string(),
                    "123456",
                    "http://localhost:3000/rdr/pprdr.asp",
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

    if let msnp11_sdk::enums::event::Event::ContactInForwardList {
        email,
        display_name,
        guid,
        groups,
        lists,
    } = client
        .add_contact(
            &"bob@passport.com".to_string(),
            &"Bob".to_string(),
            msnp11_sdk::enums::msnp_list::MsnpList::ForwardList,
        )
        .await
        .unwrap()
    {
        assert_eq!(email, "bob@passport.com");
        assert_eq!(display_name, "Bob");
        assert_eq!(guid, "6bd736b8-dc18-44c6-ad61-8cd12d641e79");
        assert_eq!(groups.len(), 0);
        assert_eq!(
            lists,
            vec![msnp11_sdk::enums::msnp_list::MsnpList::ForwardList]
        );
    }

    if let msnp11_sdk::enums::event::Event::Contact {
        email,
        display_name,
        lists,
    } = client
        .add_contact(
            &"fred@passport.com".to_string(),
            &"Fred".to_string(),
            msnp11_sdk::enums::msnp_list::MsnpList::AllowList,
        )
        .await
        .unwrap()
    {
        assert_eq!(email, "fred@passport.com");
        assert_eq!(display_name, "Fred");
        assert_eq!(
            lists,
            vec![msnp11_sdk::enums::msnp_list::MsnpList::AllowList]
        );
    }

    client.add_event_handler_closure(|event| async {
        match event {
            msnp11_sdk::enums::event::Event::AddedBy {
                email,
                display_name,
            } => {
                assert_eq!(email, "fred@passport.com");
                assert_eq!(display_name, "Fred");
            }

            _ => (),
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    client.disconnect().await.unwrap();
}
