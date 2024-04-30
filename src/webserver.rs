use std::{collections::BTreeSet, sync::Mutex};

use esp_idf_svc::{
    http::{
        self,
        server::{ws::EspHttpWsDetachedSender, EspHttpServer},
    },
    io::Write,
    sys::EspError,
    ws::FrameType,
};

use crate::dmx::DmxState;

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

    let sessions = Mutex::new(BTreeSet::<i32>::new());

    server.ws_handler("/ws/dmx", move |ws| {
        let mut sessions = sessions.lock().unwrap();

        if ws.is_new() {
            let thread_ws = ws.create_detached_sender()?;
            std::thread::spawn(move || dmx_sender(thread_ws));

            sessions.insert(ws.session());
        } else if ws.is_closed() {
            sessions.remove(&ws.session());
            return Ok::<(), EspError>(());
        }

        Ok::<(), EspError>(())
    })?;

    // keep server running beyond the scope of this function
    std::mem::forget(server);

    Ok(())
}

fn dmx_sender(mut ws: EspHttpWsDetachedSender) -> anyhow::Result<()> {
    loop {
        if ws.is_closed() {
            return Ok(());
        }

        let data = {
            let raw = DmxState::get().clone();
            raw.data
        };
        let res = ws.send(FrameType::Binary(false), data.as_slice());
        if res.is_err() {
            return Err(res.err().unwrap().into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
