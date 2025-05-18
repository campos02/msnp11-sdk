use env_logger::Env;
use msnp11_sdk::list::List;
use msnp11_sdk::event::Event;
use msnp11_sdk::client::Client;
use msnp11_sdk::models::personal_message::PersonalMessage;
use msnp11_sdk::models::presence::Presence;

#[tokio::test]
async fn login() {
    env_logger::Builder::from_env(Env::default().default_filter_or("trace")).init();
    let mut client = Client::new("127.0.0.1".to_string(), "1863".to_string())
        .await
        .unwrap();

    let result: Event = match client
        .login(
            "testing@example.com".to_string(),
            "123456".to_string(),
            "http://localhost:3000/rdr/pprdr.asp".to_string(),
        )
        .await
    {
        Ok(Event::RedirectedTo { server, port }) => {
            client = Client::new(server, port).await.unwrap();
            client
                .login(
                    "testing@example.com".to_string(),
                    "123456".to_string(),
                    "http://localhost:3000/rdr/pprdr.asp".to_string(),
                )
                .await
                .unwrap()
        }

        Ok(Event::Authenticated) => Event::Authenticated,
        Err(err) => panic!("Login error: {err}"),
        _ => Event::Disconnected,
    };

    assert_eq!(result, Event::Authenticated);

    client
        .set_presence(&Presence::new("NLN".to_string(), None))
        .await
        .unwrap();

    client
        .set_personal_message(&PersonalMessage {
            psm: "test".to_string(),
            current_media: "".to_string(),
        })
        .await
        .unwrap();

    for _ in 1..client.event_queue_size() {
        match client.receive_event().await.unwrap() {
            Event::Gtc(gtc) => assert_eq!(gtc, "A"),
            Event::Blp(blp) => assert_eq!(blp, "AL"),
            Event::DisplayName(display_name) => assert_eq!(display_name, "Testing"),

            Event::Group { name, id } => {
                assert_eq!(name, "Mock Contacts");
                assert_eq!(id, "124153dc-a695-4f6c-93e8-8e07c9775251");
            }

            Event::ContactInForwardList {
                email,
                display_name,
                id,
                lists,
                groups,
            } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(display_name, "Bob");
                assert_eq!(id, "6bd736b8-dc18-44c6-ad61-8cd12d641e79");
                assert_eq!(groups, vec!["124153dc-a695-4f6c-93e8-8e07c9775251"]);
                assert_eq!(
                    lists,
                    vec![List::ForwardList, List::BlockList, List::ReverseList]
                )
            }

            Event::Contact {
                email,
                display_name,
                lists,
            } => {
                assert_eq!(email, "fred@passport.com");
                assert_eq!(display_name, "Fred");
                assert_eq!(lists, vec![List::AllowList])
            }

            Event::PresenceUpdate { email, display_name, presence } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(display_name, "Bob");
                assert_eq!(presence, Presence {
                    presence: "NLN".to_string(),
                    client_id: 1073741824,
                    msn_object: Some("<msnobj Creator=\"".to_string())
                });
            }

            Event::PersonalMessageUpdate { email, personal_message } => {
                assert_eq!(email, "bob@passport.com");
                assert_eq!(personal_message, PersonalMessage {
                    psm: "my msn all ducked".to_string(),
                    current_media: "".to_string()
                });
            }

            _ => (),
        }
    }
}