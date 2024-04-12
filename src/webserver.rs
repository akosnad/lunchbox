use esp_idf_svc::{
    http::{self, server::EspHttpServer},
    io::Write,
};

use crate::artnet::DmxState;

static WEB_FILES: &[(&str, &[u8])] = &include!(concat!(env!("OUT_DIR"), "/web_files.rs"));

pub fn init() -> anyhow::Result<()> {
    let mut server = {
        let config = http::server::Configuration {
            stack_size: 10240,
            ..Default::default()
        };
        EspHttpServer::new(&config)?
    };

    for (name, data) in WEB_FILES {
        let content_type = if name.ends_with(".html") {
            "text/html"
        } else if name.ends_with(".css") {
            "text/css"
        } else if name.ends_with(".js") {
            "application/javascript"
        } else {
            "application/octet-stream"
        };

        let name = if *name == "/index.html" { "/" } else { *name };

        server.fn_handler(name, http::Method::Get, move |req| {
            req.into_response(
                200,
                Some("OK"),
                &[
                    ("Content-Type", content_type),
                    ("Access-Control-Allow-Origin", "*"),
                ],
            )?
            .write_all(data)
            .map(|_| ())
        })?;
    }

    server.fn_handler("/dmx", http::Method::Get, |req| {
        let data = {
            let raw = DmxState::get().clone();
            raw.data
            //.as_slice()
            //.into_iter()
            //.map(|x| format!("{:02X}", x).as_bytes())
            //.flatten()
            //.collect::<Vec<_>>()
            //.as_slice()
        };
        req.into_response(
            200,
            Some("OK"),
            &[
                ("Content-Type", "application/octet-stream"),
                ("Access-Control-Allow-Origin", "*"),
            ],
        )?
        .write_all(data.as_slice())
        .map(|_| ())
    })?;

    // keep server running beyond the scope of this function
    std::mem::forget(server);

    Ok(())
}
