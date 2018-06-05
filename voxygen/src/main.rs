#![feature(nll)]

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate get_if_addrs;
#[macro_use]
extern crate enum_map;
extern crate nalgebra;

extern crate client;
extern crate region;

mod game;
mod window;
mod renderer;
mod mesh;
mod model_object;
mod pipeline;
mod camera;
mod render_volume;

use std::io;
use std::net::SocketAddr;

use client::ClientMode;
use game::Game;

fn main() {
    println!("Starting Voxygen...");

    let ip = get_if_addrs::get_if_addrs().unwrap()[0].ip();
    let port: u16 = 59001;

    println!("Binding local port to {}:{}...", ip.to_string(), port);

    let mut remote_addr = String::new();
    println!("Remote server address [127.0.0.1:59003]:");
    io::stdin().read_line(&mut remote_addr).unwrap();
    remote_addr = remote_addr.trim().to_string();
    if remote_addr.len() == 0 {
        remote_addr = "127.0.0.1:59003".to_string();
    }

    let game = Game::new(
        ClientMode::Character,
        &"voxygen-test",
        SocketAddr::new(ip, port),
        remote_addr
    ).run();
}
