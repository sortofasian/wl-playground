use std::fs::File;
use std::io::{BufWriter, Write};
use std::os::unix::prelude::AsRawFd;
use std::process::exit;

use wayland_client::{
    protocol::{wl_compositor, wl_shm},
    Display, GlobalManager, Interface,
};

use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_wm_base, xdg_toplevel};

fn main() {
    let display = Display::connect_to_env().expect("Could not connect to wayland");
    let mut queue = display.create_event_queue();
    let display = (*display).attach(queue.token());
    let globals = GlobalManager::new(&display);
    queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();

    let x = 240;
    let y = 320;
    let mut tmp = tempfile::tempfile().expect("could not create tempfile");
    draw(&mut tmp, x, y);

    let compositor = globals
        .instantiate_exact::<wl_compositor::WlCompositor>(wl_compositor::WlCompositor::VERSION)
        .expect("Could not bind compositor");

    let shm = globals
        .instantiate_exact::<wl_shm::WlShm>(wl_shm::WlShm::VERSION)
        .expect("Could not bind shm");

    let xdg_base = globals
        .instantiate_exact::<xdg_wm_base::XdgWmBase>(xdg_wm_base::XdgWmBase::VERSION)
        .expect("Could not bind xdg");

    let surface = compositor.create_surface();
    let xdg_surface = xdg_base.get_xdg_surface(&surface);
    let xdg_toplevel = xdg_surface.get_toplevel();

    let pool = shm.create_pool(tmp.as_raw_fd(), (x * y * 4) as i32);
    let buf = pool.create_buffer(0, x, y, x * 4, wl_shm::Format::Argb8888);


    xdg_base.quick_assign(|xdg, event, _| {
        if let xdg_wm_base::Event::Ping { serial } = event {
            xdg.pong(serial);
        }
    });

    xdg_surface.quick_assign(|xdg_surface, event, _| {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
        }
    });

    xdg_toplevel.quick_assign(|_, event, _| match event {
        xdg_toplevel::Event::Close => {
            exit(0)
        }
        _ => {}
    });


    surface.commit();
    queue
        .sync_roundtrip(&mut (), |_, _, _| {})
        .unwrap();

    surface.attach(Some(&buf), 0, 0);
    surface.commit();

    loop {
        queue.dispatch(&mut (), |_, _, _| {}).unwrap();
    }
}

fn draw(tmp: &mut File, x: i32, y: i32) -> () {
    let mut buf = BufWriter::new(tmp);
    for cur_y in 0..y {
        for cur_x in 0..x {
            let a = 0xFF;
            let r = (((x - cur_x) * 0xFF) / x).min(((y - cur_y) * 0xFF) / y);
            let g = ((cur_x * 0xFF) / x).min(((y - cur_y) * 0xFF) / y);
            let b = (((x - cur_x) * 0xFF) / x).min((cur_y * 0xFF) / y);
            let color: i32 = (a << 24) + (r << 16) + (g << 8) + b;
            buf.write_all(&color.to_ne_bytes()).unwrap();
        }
    }
    buf.flush().unwrap();
}
