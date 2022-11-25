#![windows_subsystem = "windows"]

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use nwd::NwgUi;
use nwg::NativeUi;
use rocket::fs::FileServer;

use std::{
    cell::{Cell, RefCell},
    path,
};

fn start_server(
    path: String,
) -> (
    rocket::tokio::sync::oneshot::Sender<bool>,
    std::thread::JoinHandle<()>,
) {
    let (tx, rx) = rocket::tokio::sync::oneshot::channel();
    let t = std::thread::spawn(move || {
        let path = path;
        rocket::tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let config = rocket::Config {
                    port: 9234,
                    address: std::net::Ipv4Addr::new(127, 0, 0, 1).into(),
                    ..Default::default()
                };
                let mut rocket = rocket::build()
                    .configure(config)
                    .mount("/", FileServer::from(&path));
                if !find_index(&path) {
                    rocket = rocket.mount("/", rocket::routes![dir_list]);
                }
                let rocket = rocket.launch();
                rocket::tokio::select! {
                    _ = rx => {
                        println!("Server Stop...");
                    }
                    _ = rocket => {

                    }
                }
            })
    });
    (tx, t)
}

#[rocket::get("/")]
fn dir_list() -> String {
    let mut result = "<pre>\n".to_string();

    result.push_str("</pre>\n");
    result
}

fn find_index(root: &str) -> bool {
    let path = std::path::Path::new(root);
    for entry in path.read_dir().unwrap() {
        if let Ok(entry) = entry {
            if entry.path().file_name().unwrap() == "index.html" {
                return true;
            }
        }
    }
    false
}

#[derive(Default, NwgUi)]
pub struct StaticFileServerApp {
    #[nwg_control(size: (500, 100), position: (200, 300), title: "Static File Server", accept_files: true)]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()], OnFileDrop: [StaticFileServerApp::specify_serve_path(SELF, EVT_DATA)] )]
    window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 1)]
    grid: nwg::GridLayout,

    #[nwg_control(text: "DRAG AND DROP FOLDER HERE")]
    #[nwg_layout_item(layout: grid, row: 0, col: 0)]
    text: nwg::Label,

    #[nwg_control(text: "Start", enabled: false)]
    #[nwg_layout_item(layout: grid, row: 1, col: 0)]
    #[nwg_events( OnButtonClick: [StaticFileServerApp::start_serving(SELF)] )]
    start_button: nwg::Button,

    path: RefCell<Option<String>>,
    running: Cell<bool>,
    shutdown: RefCell<Option<rocket::tokio::sync::oneshot::Sender<bool>>>,
    server_thread: RefCell<Option<std::thread::JoinHandle<()>>>,
}

impl StaticFileServerApp {
    pub fn specify_serve_path(&self, data: &nwg::EventData) {
        if self.running.get() {
            nwg::simple_message("错误", "请先停止服务器");
            return;
        }
        let drop = data.on_file_drop();

        if let Some(directory) = drop.files().get(0) {
            let d = path::Path::new(&directory);
            if !d.is_dir() {
                nwg::simple_message("错误", "需要一个文件夹");
                return;
            }
            self.text.set_text(&directory);
            self.path.replace(Some(directory.clone()));
            self.start_button.set_enabled(true);
        } else {
        }
    }

    pub fn start_serving(&self) {
        if self.running.get() {
            let shutdown = self.shutdown.replace(None);
            if let Some(shutdown) = shutdown {
                shutdown.send(true).unwrap();
            }
            let server_thread = self.server_thread.replace(None);
            if let Some(server_thread) = server_thread {
                server_thread.join().unwrap();
            }
            self.start_button.set_text("Start");
        } else {
            let path = self.path.borrow().clone().unwrap();
            let (tx, t) = start_server(path);
            self.server_thread.replace(Some(t));
            self.shutdown.replace(Some(tx));
            self.start_button
                .set_text("Running at http://127.0.0.1:9234 > Stop");
            let _ = std::process::Command::new("cmd.exe")
                .arg("/C")
                .arg("start")
                .arg("")
                .arg("http://127.0.0.1:9234")
                .spawn()
                .expect("failed to launch browser");
        }
        self.running.replace(!self.running.get());
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("SimSun").expect("Failed to set default font");

    let _app = StaticFileServerApp::build_ui(Default::default()).expect("Failed to build UI");

    nwg::dispatch_thread_events();
}
