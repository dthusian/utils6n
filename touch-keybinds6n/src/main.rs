use std::collections::HashMap;
use std::{env, fs};
use std::env::args;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::{c_char, c_int, c_uchar, c_uint, c_ulong};
use std::ptr::{null, null_mut, slice_from_raw_parts};
use std::thread::{sleep, yield_now};
use std::time::Duration;
use x11::xlib;
use serde::{Serialize, Deserialize};

fn get_config_path() -> Result<String, anyhow::Error> {
  match env::var("XDG_CONFIG_HOME") {
    Ok(v) => Result::Ok(v + "/touch-keybind6n.yaml"),
    Err(_) => {
      match env::var("HOME") {
        Ok(v) => Result::Ok(v + "/.config/touch-keybind6n.yaml"),
        Err(_) => Result::Err(anyhow::Error::msg("Config file not found"))
      }
    }
  }
}

unsafe extern "C" fn _err_handler(display: *mut xlib::Display, err: *mut xlib::XErrorEvent) -> c_int {
  panic!("Xlib error: {}", (*err).error_code);
}

unsafe fn check_window_name(name_arg: &str, display: *mut xlib::Display, wnd: xlib::Window) -> bool {
  let mut name: *mut c_char = null_mut();
  xlib::XFetchName(display, wnd, &mut name as *mut *mut c_char);
  if name == null_mut() { return false; }
  let name_len;
  let mut j = 0;
  loop {
    if *name.offset(j) == '\0' as c_char {
      name_len = j;
      break;
    }
    j += 1;
  }
  let b = String::from_utf8_lossy(&*slice_from_raw_parts(name as *const u8, name_len as usize)).contains(name_arg);
  xlib::XFree(name as *mut c_void);
  b
}

unsafe fn find_window_by_name(name_arg: &str, display: *mut xlib::Display, search_under: xlib::Window) -> Option<xlib::Window> {
  let mut _1: xlib::Window = 0;
  let mut _2: xlib::Window = 0;
  let mut children: *mut xlib::Window = null_mut();
  let mut n_children: u32 = 0;
  xlib::XQueryTree(display, search_under, &mut _1 as *mut xlib::Window, &mut _2 as *mut xlib::Window, &mut children as *mut *mut xlib::Window, &mut n_children as *mut u32);
  if n_children == 0 {
    return None;
  }
  for i in 0..n_children {
    let child = *children.offset(i as isize);
    if check_window_name(name_arg, display, child) {
      xlib::XFree(children as *mut c_void);
      return Some(child);
    }
    match find_window_by_name(name_arg, display, child) {
      Some(wnd) => {
        xlib::XFree(children as *mut c_void);
        return Some(wnd)
      }
      None => {}
    }
  }
  xlib::XFree(children as *mut c_void);
  None
}

fn setup_event_loop<F>(window_name: &str, callback: F) -> !
  where F: Fn(&str, (i64, i64), (i64, i64)) -> (i64, i64)
{
  unsafe {
    // Setup display and find relevant window
    xlib::XSetErrorHandler(Some(_err_handler));
    let display = xlib::XOpenDisplay(null());
    let mut wnd = None;
    for screen_id in 0..xlib::XScreenCount(display) {
      let screen = xlib::XScreenOfDisplay(display, screen_id);
      let maybe_wnd = find_window_by_name(window_name, display, xlib::XRootWindowOfScreen(screen));
      if maybe_wnd.is_some() {
        wnd = Some(maybe_wnd.unwrap());
        break;
      }
    }
    if wnd == None { panic!("Window not found") }
    let wnd = wnd.unwrap();
    xlib::XSelectInput(display, wnd, xlib::KeyPressMask);
    loop {
      while xlib::XPending(display) != 0 {
        let mut xe: xlib::XEvent;
        xe = xlib::XEvent {
          key: xlib::XKeyEvent {
            type_: 0,
            serial: 0,
            send_event: 0,
            display,
            window: 0,
            root: 0,
            subwindow: 0,
            time: 0,
            x: 0,
            y: 0,
            x_root: 0,
            y_root: 0,
            state: 0,
            keycode: 0,
            same_screen: 0
          }
        };
        xlib::XNextEvent(display, &mut xe as *mut xlib::XEvent);
        if xe.type_ == xlib::KeyPress {
          let keysym = xlib::XKeycodeToKeysym(display, xe.key.keycode as c_uchar, 0);
          let key: u8;
          if keysym >= x11::keysym::XK_0 as c_ulong && keysym <= x11::keysym::XK_9 as c_ulong {
            key = (keysym - x11::keysym::XK_0 as c_ulong) as u8 + '0' as u8;
          } else if keysym >= x11::keysym::XK_A as c_ulong && keysym <= x11::keysym::XK_Z as c_ulong {
            key = (keysym - x11::keysym::XK_A as c_ulong) as u8 + 'A' as u8;
          } else if keysym >= x11::keysym::XK_a as c_ulong && keysym <= x11::keysym::XK_z as c_ulong {
            key = (keysym - x11::keysym::XK_a as c_ulong) as u8 + 'A' as u8;
          } else {
            key = 0;
          }
          if key != 0 {
            let mut root: xlib::Window = 0;
            let mut _x: c_int = 0;
            let mut _y: c_int = 0;
            let mut w: c_uint = 0;
            let mut h: c_uint = 0;
            let mut _1: c_uint = 0;
            let mut _2: c_uint = 0;
            xlib::XGetGeometry(
              display, wnd, &mut root as *mut xlib::Window,
              &mut _x as *mut c_int, &mut _x as *mut c_int,
              &mut w as *mut c_uint, &mut h as *mut c_uint,
              &mut _1 as *mut c_uint, &mut _2 as *mut c_uint,
            );
            let wnd_size = (w as i64, h as i64);
            xlib::XGetGeometry(
              display, root, &mut root as *mut xlib::Window,
              &mut _x as *mut c_int, &mut _x as *mut c_int,
              &mut w as *mut c_uint, &mut h as *mut c_uint,
              &mut _1 as *mut c_uint, &mut _2 as *mut c_uint,
            );
            let screen_size = (w as i64, h as i64);
            let click_coord = callback(&String::from(key as char), wnd_size, screen_size);
            if click_coord.0 > 0 && click_coord.1 > 0 {
              let mut ev = xlib::XEvent {
                button: xlib::XButtonEvent {
                  type_: xlib::ButtonPress,
                  serial: 0,
                  send_event: 1,
                  display,
                  window: wnd,
                  root,
                  subwindow: 0,
                  time: xe.key.time,
                  x: click_coord.0 as c_int,
                  y: click_coord.1 as c_int,
                  x_root: xe.key.x_root,
                  y_root: xe.key.x_root,
                  state: xe.key.state,
                  same_screen: 1,
                  button: xlib::Button1
                }
              };
              xlib::XSendEvent(display, wnd, 1, xlib::KeyPressMask, &mut ev as *mut xlib::XEvent);
              println!("Key '{}' -> Click ({}, {})", key as char, click_coord.0, click_coord.1);
            }
          }
        }
      }
      sleep(Duration::from_millis(10));
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub keybinds: HashMap<String, (f64, f64)>,
  pub chrome_size: (i64, i64, i64, i64) // up, down, left, right
  // chrome is ui elements that take up space only when not fullscreened
}

fn main() -> ! {
  let config: Config = serde_yaml::from_str(
    &fs::read_to_string(
      get_config_path().unwrap()
    ).unwrap()
  ).unwrap();
  let arg: String = args().collect::<Vec<String>>()[1].clone();
  println!("Chrome size: Top: {} Bottom: {} Left: {} Right: {}", config.chrome_size.0, config.chrome_size.1, config.chrome_size.2, config.chrome_size.3);
  setup_event_loop(&arg, move |k: &str, dimensions: (i64, i64), screen_size: (i64, i64)| {
    //eprintln!("[debug] wnd size: {}x{}", dimensions.0, dimensions.1);
    if let Some(coords) = config.keybinds.get(k) {
      if dimensions != screen_size {
        (
          ((dimensions.0 - config.chrome_size.2 - config.chrome_size.3) as f64 * coords.0 + config.chrome_size.2 as f64).floor() as i64,
          ((dimensions.1 - config.chrome_size.0 - config.chrome_size.1) as f64 * coords.1 + config.chrome_size.0 as f64).floor() as i64
        )
      } else {
        (
          (dimensions.0 as f64 * coords.0).floor() as i64,
          (dimensions.1 as f64 * coords.1).floor() as i64
        )
      }
    } else {
      (-1, -1)
    }
  });
}
