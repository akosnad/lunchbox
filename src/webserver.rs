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

pub fn init(dmx_state: DmxState) -> anyhow::Result<()> {
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

    let dmx_state_1 = dmx_state.clone();
    server.fn_handler("/dmx", http::Method::Get, move |req| {
        let data = dmx_state_1.get();
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

    let dmx_state_2 = dmx_state.clone();
    server.ws_handler("/ws/dmx", move |ws| {
        let mut sessions = sessions.lock().unwrap();

        if ws.is_new() {
            let thread_ws = ws.create_detached_sender()?;
            let dmx_state_clone = dmx_state_2.clone();
            std::thread::spawn(move || dmx_sender(thread_ws , dmx_state_clone));

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

fn dmx_sender(mut ws: EspHttpWsDetachedSender, dmx_state: DmxState) -> anyhow::Result<()> {
    loop {
        if ws.is_closed() {
            return Ok(());
        }

        let data = dmx_state.get();
        let res = ws.send(FrameType::Binary(false), data.as_slice());
        if res.is_err() {
            return Err(res.err().unwrap().into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
