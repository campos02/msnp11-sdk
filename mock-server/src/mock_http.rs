use axum::Router;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use hyper::Request;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server;
use log::error;
use tower_service::Service;

pub struct MockHttp;

impl MockHttp {
    pub async fn mock_passport() {
        let app = Router::new()
            .route("/rdr/pprdr.asp", get(Self::nexus))
            .route("/login.srf", get(Self::login_srf))
            .route("/Config/MsgrConfig.asmx", post(Self::config));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .expect("Could not bind HTTP server");

        loop {
            let (socket, _remote_addr) = match listener.accept().await {
                Ok(l) => l,
                Err(error) => {
                    error!(": {error}");
                    continue;
                }
            };

            let tower_service = app.clone();

            tokio::spawn(async move {
                let socket = TokioIo::new(socket);
                let hyper_service =
                    hyper::service::service_fn(move |request: Request<Incoming>| {
                        tower_service.clone().call(request)
                    });

                let mut builder = server::conn::auto::Builder::new(TokioExecutor::new());
                builder.http1().title_case_headers(true);

                if let Err(err) = builder
                    .serve_connection_with_upgrades(socket, hyper_service)
                    .await
                {
                    error!("Failed to serve connection: {err:#}");
                }
            });
        }
    }

    async fn nexus() -> impl IntoResponse {
        [("PassportURLs", "DALogin=http://localhost:3000/login.srf")]
    }

    async fn login_srf() -> impl IntoResponse {
        [(
            "Authentication-Info",
            "Passport1.4 da-status=success,from-PP='aaa123aaa123'",
        )]
    }

    async fn config() -> impl IntoResponse {
        // Response taken from Crosstalk
        "<?xml version=\"1.0\" encoding=\"utf-8\" ?>
        <soap:Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\" \
                       xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
                       xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\">
            <soap:Body>
                <GetClientConfigResponse xmlns=\"http://www.msn.com/webservices/Messenger/Client\">
                    <GetClientConfigResult>
                        <![CDATA[
                        <MsgrConfig>
                        <Simple>
                        <Config>
                        <ExpiresInDays>0</ExpiresInDays>
                        </Config>
                        <DisablePhoneDialer>1</DisablePhoneDialer>
                        <MinFlashPlayer BuildNumber=\"60\" MajorVersion=\"7\" MinorVersion=\"0\"></MinFlashPlayer>
                        <Relay>
                        <Enabled>0</Enabled>
                        <MaxCallLength>0</MaxCallLength>
                        </Relay>
                        <TrustedDomains>
                        <domain name=\"hiden.cc\"/>
                        <domain name=\"ugnet.gay\"/>
                        <domain name=\"crosstalk.im\"/>
                        <domain name=\"ctsrv.gay\"/>
                        </TrustedDomains>
                        <ErrorResponseTable>
                        <Feature type=\"0\" name=\"Login\">
                        <Entry hr=\"0x80072EE7\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000306\" action=\"3\"></Entry>
                        <Entry hr=\"0x80072EFD\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000362\" action=\"3\"></Entry>
                        <Entry hr=\"0x8100030E\" action=\"3\"></Entry>
                        <Entry hr=\"0x80072745\" action=\"3\"></Entry>
                        <Entry hr=\"0x800701F7\" action=\"3\"></Entry>
                        <Entry hr=\"0x80072EFF\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000363\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000395\" action=\"3\"></Entry>
                        <Entry hr=\"0x800B0001\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000323\" action=\"3\"></Entry>
                        <Entry hr=\"0x80072F19\" action=\"3\"></Entry>
                        <Entry hr=\"0x800701F8\" action=\"3\"></Entry>
                        <Entry hr=\"0x80072746\" action=\"3\"></Entry>
                        <Entry hr=\"0x800701F6\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000377\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000314\" action=\"3\"></Entry>
                        <Entry hr=\"0x81000378\" action=\"3\"></Entry>
                        <Entry hr=\"0x80072EFF\" action=\"3\"></Entry>
                        <Entry hr=\"0x80070190\" action=\"3\"></Entry>
                        <Entry hr=\"0x80070197\" action=\"3\"></Entry>
                        <Entry hr=\"0x80048820\" action=\"3\"></Entry>
                        <Entry hr=\"0x80048829\" action=\"3\"></Entry>
                        <Entry hr=\"0x80048834\" action=\"3\"></Entry>
                        <Entry hr=\"0x80048852\" action=\"3\"></Entry>
                        <Entry hr=\"0x8004886a\" action=\"3\"></Entry>
                        <Entry hr=\"0x8004886b\" action=\"3\"></Entry>
                        </Feature>
                        <Feature type=\"2\" name=\"MapFile\">
                        <Entry hr=\"0x810003F0\" action=\"3\"></Entry>
                        <Entry hr=\"0x810003F1\" action=\"3\"></Entry>
                        <Entry hr=\"0x810003F2\" action=\"3\"></Entry>
                        <Entry hr=\"0x810003F3\" action=\"3\"></Entry>
                        <Entry hr=\"0x810003F4\" action=\"3\"></Entry>
                        <Entry hr=\"0x810003F5\" action=\"3\"></Entry>
                        <Entry hr=\"0x810003F6\" action=\"3\"></Entry>
                        <Entry hr=\"0x810003F7\" action=\"3\"></Entry>
                        </Feature>
                        </ErrorResponseTable>
                        </Simple>
                        <TabConfig>
                        <msntabsettings>
                        <oemtotallimit>1</oemtotallimit>
                        <oemdisplaylimit>1</oemdisplaylimit>
                        </msntabsettings>
                        <msntabdata>

                        <tab>
                        <type>page</type>
                        <contenturl>http://wiby.me/</contenturl>
                        <hiturl>http://wiby.me/</hiturl>
                        <image>http://static.ugnet.gay/svc/tab/wiby.png</image>
                        <name>Wiby</name>
                            <tooltip>Wiby</tooltip>
                            <siteid>0</siteid>
                            <notificationid>0</notificationid>
                            </tab>

                            <tab>
                            <type>page</type>
                        <contenturl>http://frogfind.com/</contenturl>
                        <hiturl>http://frogfind.com/</hiturl>
                        <image>http://static.ugnet.gay/svc/tab/frogfind.png</image>
                        <name>FrogFind!</name>
                            <tooltip>FrogFind!</tooltip>
                            <siteid>0</siteid>
                            <notificationid>0</notificationid>
                            </tab>

                            <tab>
                            <type>page</type>
                        <contenturl>http://theoldnet.com</contenturl>
                        <hiturl>http://theoldnet.com</hiturl>
                        <image>http://static.ugnet.gay/svc/tab/theoldnet.png</image>
                        <name>TheOldNet</name>
                            <tooltip>TheOldNet</tooltip>
                            <siteid>0</siteid>
                            <notificationid>0</notificationid>
                            </tab>

                            </msntabdata>
                            </TabConfig>
                            <AbchCfg>
                            <abchconfig>
                            <url>https://ctsvcs.addressbook.ugnet.gay/abservice.asmx</url>
                        </abchconfig>
                            </AbchCfg>
                            <LocalizedConfig Market=\"en-US\">
                            <AdMainConfig>
                            <TextAdRefresh>1</TextAdRefresh>
                            <TextAdServer>http://ctsvcs.advertising.ugnet.gay/ads/txt</TextAdServer>
                        <AdBanner20URL Refresh=\"300\">http://ctsvcs.advertising.ugnet.gay/ads/banner</AdBanner20URL>
                        </AdMainConfig>
                            <AppDirConfig>
                            <AppDirPageURL>http://mactivities.msgrsvcs.ctsrv.gay/AppDirectory/Directory.aspx?L=en-us</AppDirPageURL>
                        <AppDirSeviceURL>http://mactivities.msgrsvcs.ctsrv.gay/AppDirectory/AppDirectory.asmx</AppDirSeviceURL>
                        <AppDirVersionURL>http://mactivities.msgrsvcs.ctsrv.gay/AppDirectory/GetAppdirVersion.aspx</AppDirVersionURL>
                        </AppDirConfig>
                            <MSNSearch>
                            <DesktopInstallURL>https://searx.ugnet.xyz/searxng/search?q=$QUERY$&amp;ia=web</DesktopInstallURL>
                        <ImagesURL>https://searx.ugnet.xyz/searxng/search?q=$QUERY$&amp;ia=images</ImagesURL>
                        <NearMeURL>https://searx.ugnet.xyz/searxng/search?q=$QUERY$&amp;ia=web</NearMeURL>
                        <NewsURL>https://searx.ugnet.xyz/searxng/search?q=$QUERY$+news&amp;ia=news</NewsURL>
                        <SearchKidsURL>https://searx.ugnet.xyz/searxng/search?q=$QUERY$&amp;ia=web</SearchKidsURL>
                        <SearchURL>https://searx.ugnet.xyz/searxng/search?q=$QUERY$&amp;ia=web</SearchURL>
                        <SharedSearchURL>https://searx.ugnet.xyz/searxng/search?q=$QUERY$&amp;ia=web</SharedSearchURL>
                        <SharedSearchURL2>https://searx.ugnet.xyz/searxng/search?q=$QUERY$&amp;ia=web</SharedSearchURL2>
                        </MSNSearch>
                            <MsnTodayConfig>
                            <MsnTodayURL>http://today.msgrsvcs.ctsrv.gay/start?msn=1</MsnTodayURL>
                        </MsnTodayConfig>
                            <MusicIntegration URL=\"https://www.last.fm/search/tracks?q=$ARTIST$+$TITLE$\"/>
                            <RL>
                            <ViewProfileURL>https://crosstalk.im/account/settings</ViewProfileURL>
                        </RL>
                            <TermsOfUse>
                            <TermsOfUseSID>956</TermsOfUseSID>
                            <TermsOfUseURL>https://crosstalk.im/tos</TermsOfUseURL>
                        </TermsOfUse>
                            </LocalizedConfig>
                            </MsgrConfig>
                        ]]>
                    </GetClientConfigResult>
                </GetClientConfigResponse>
            </soap:Body>
        </soap:Envelope>"
    }
}
