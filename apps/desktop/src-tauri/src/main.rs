// リリースビルドの Windows でコンソールウィンドウを出さない。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    runnerdock_desktop_lib::run();
}
