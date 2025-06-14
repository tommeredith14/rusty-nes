mod nes;
use std::time::{self, Duration, Instant};

use iced::{self, keyboard}; //, subscription};

use image::{DynamicImage, EncodableLayout};

use crate::nes::Nes;

use iced::{Element, Subscription, widget};

pub fn main() -> iced::Result {
    // let mut nes = Nes::new();
    // nes.load_rom(String::from("donkey_kong.nes"));
    // nes.ppu.borrow().render_chr();
    iced::application("RustyNES", update, view)
        .subscription(subscription)
        .run()
    // IcedApp::run(Settings::default());
    // let native_options = eframe::NativeOptions::default();
    // eframe::run_native("My egui App",
    //     native_options,
    //     Box::new(|cc| {
    //         Box::<MyEguiApp>::default()
    //     }),
    // );
}

struct IcedApp {
    nes: Nes,

    // state
    frame_rate: f64,
    controller_state: IcedControllerState,

    // cached images
    chr_image: Option<image::RgbaImage>,
    nt_image: Option<image::RgbaImage>,
    frame: image::RgbaImage,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    RefreshChrPressed,
    Tick(Instant),
    KeyPress(iced::keyboard::Key),
    KeyReleased(iced::keyboard::Key),
    // Event(iced::Event)
}

impl Default for IcedApp {
    fn default() -> Self {
        IcedApp::new(())
    }
}

impl IcedApp {
    // type Executor = executor::Default;
    // type Flags = ();
    // type Message = AppMessage;
    // type Theme = Theme;

    fn new(_flags: ()) -> IcedApp {
        let mut nes = Nes::default();
        //nes.load_rom(String::from("donkey_kong.nes"));
        //  nes.load_rom(String::from("super_mario_brothers.nes"));
        _ = nes.load_rom(String::from("super_mario_brothers.nes"));
        // nes.load_rom(String::from("nes-test-roms/full_palette/full_palette.nes"));
        // nes.load_rom(String::from("nes-test-roms/ppu_vbl_nmi/rom_singles/01-vbl_basics.nes"));
        IcedApp {
            nes,
            chr_image: None,
            nt_image: None,
            frame_rate: 60.0,
            frame: image::RgbaImage::new(256, 240),
            controller_state: IcedControllerState::default(),
        }
    }
}

fn subscription(state: &IcedApp) -> Subscription<AppMessage> {
    if true {
        //self.is_playing {
        Subscription::batch([
            iced::time::every(Duration::from_millis(1000 / state.frame_rate as u64))
                .map(AppMessage::Tick),
            keyboard::on_key_press(|key, _modifiers| Some(AppMessage::KeyPress(key))),
            keyboard::on_key_release(|key, _modifiers| Some(AppMessage::KeyReleased(key))),
            // subscription::events().map(AppMessage::Event)
        ])
        //iced::event::listen().map(Self.Message::Event)
    } else {
        Subscription::none()
    }
}

fn update(state: &mut IcedApp, message: AppMessage) {
    match message {
        AppMessage::RefreshChrPressed => {
            let chr_image = state.nes.ppu.borrow().render_chr();
            state.chr_image = Some(DynamicImage::ImageLuma8(chr_image).into_rgba8());
            let nt_image = state.nes.ppu.borrow().render_nt();
            state.nt_image = Some(DynamicImage::ImageRgb8(nt_image).into_rgba8());
            state.nes.ppu.borrow().print_nametable();
        }
        AppMessage::Tick(_instant) => {
            println!("Frame update");
            let t = time::SystemTime::now();
            state
                .nes
                .inputs
                .borrow_mut()
                .set_controller1_state(state.controller_state.state);
            state.frame = DynamicImage::ImageRgb8(state.nes.run_frame()).into_rgba8();
            let d = t.elapsed();
            println!("Took {}s", d.unwrap().as_millis());
        }
        AppMessage::KeyPress(key) => state.controller_state.on_event(key, true),
        AppMessage::KeyReleased(key) => state.controller_state.on_event(key, false),
    }
}

fn view(state: &IcedApp) -> Element<AppMessage> {
    // "Hello, world!".into();
    // let chr_image = self.nes.ppu.borrow().render_chr();
    // let chr_image = DynamicImage::ImageLuma8(chr_image).into_rgba8().as_bytes().to_owned();
    let nt_image = if let Some(image) = state.nt_image.clone() {
        widget::image::Handle::from_rgba(image.width(), image.height(), image.as_bytes().to_owned())
    } else {
        let data = vec![0u8; 200 * 200 * 4];
        widget::image::Handle::from_rgba(200, 200, data)
    };
    let chr_image = if let Some(image) = state.chr_image.clone() {
        widget::image::Handle::from_rgba(image.width(), image.height(), image.as_bytes().to_owned())
    } else {
        let data = vec![0u8; 200 * 200 * 4];
        widget::image::Handle::from_rgba(200, 200, data)
    };
    let frame = state.frame.clone();
    let frame = widget::image::Handle::from_rgba(
        frame.width(),
        frame.height(),
        frame.as_bytes().to_owned(),
    );
    // widget::image::Handle::from_pixels(144,171,Some(chr_image);
    let content = iced::widget::column![
        widget::text(String::from("NES Screen"))
            .size(30)
            .width(iced::Length::Fill),
        widget::image::viewer(frame)
            .max_scale(1.0)
            .min_scale(2.0)
            .width(iced::Length::Fill),
        widget::text(String::from("CHR Data"))
            .size(30)
            .width(iced::Length::Fill),
        widget::image::viewer(chr_image)
            .max_scale(2.0)
            .min_scale(2.0)
            .width(iced::Length::Fill),
        widget::image::viewer(nt_image)
            .max_scale(2.0)
            .min_scale(2.0)
            .width(iced::Length::Fill),
        widget::button("Refresh").on_press(AppMessage::RefreshChrPressed)
    ];
    iced::widget::container(content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .center_x(iced::Length::Fill)
        .center_y(iced::Length::Fill)
        .into()
}

#[derive(Default)]
struct IcedControllerState {
    state: nes::input::ControllerState,
}

impl IcedControllerState {
    fn on_event(&mut self, key: iced::keyboard::Key, active: bool) {
        println!("Got key {:?} {active}", key);
        use keyboard::Key;
        use keyboard::key::Named;

        match key {
            Key::Named(name) => match name {
                Named::ArrowUp => self.state.up = active,
                Named::ArrowDown => self.state.down = active,
                Named::ArrowLeft => self.state.left = active,
                Named::ArrowRight => self.state.right = active,
                Named::Enter => self.state.start = active,
                Named::Shift => self.state.select = active,
                _ => (),
            },
            Key::Character(c) => match c.as_str() {
                "x" => self.state.a = active,
                "z" => self.state.b = active,
                _ => (),
            },
            _ => {}
        }
    }
}

// fn main() {
//     println!("Welcome to Tom's NES Emulator");
//     let mut nes = Nes::new();
//     // nes.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/01-basics.nes"));
//     // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/02-implied.nes"));
//     // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/03-immediate.nes"));
//     // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/14-rti.nes"));
//     // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/15-brk.nes"));
//     // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/16-special.nes"));
//     nes.load_rom(String::from("nes-test-roms/instr_test-v5/official_only.nes"));
//     // nes.load_rom(String::from("donkey_kong.nes"));
//     for _ in 1..20000000 {
//         let (_count, done) = nes.cpu.borrow_mut().run_instruction();
//         if done {
//             println!("Loop detected");
//             break;
//         }
//         // if i % 100 == 0 {
//         // println!();
//         // println!("******************************");
//         // println!("***     PROGRESS UPDATE    ***");
//         // println!("Test status: {}, {:x}, {:x},{:x}", cpu.memory.read_byte(0x6000), cpu.memory.read_byte(0x6001), cpu.memory.read_byte(0x6002), cpu.memory.read_byte(0x6003));
//         // println!("Test output:");
//         // for i in 0..100 {
//         //     let c = cpu.memory.read_byte(0x6004+i) as char;
//         //     // if c != '\0' {
//         //         print!("{}", c);
//         //     // } else {
//         //         // break;
//         //     // }
//         // }
//         // println!("******************************");
//         // }
//     }
//     // Test result
//     println!(
//         "Test result: {}, {:x}, {:x},{:x}",
//         nes.mem.borrow().read_byte(0x6000),
//         nes.mem.borrow().read_byte(0x6001),
//         nes.mem.borrow().read_byte(0x6002),
//         nes.mem.borrow().read_byte(0x6003)
//     );
//     println!("Test output:");
//     for i in 0..100 {
//         print!("{}", nes.mem.borrow().read_byte(0x6004 + i) as char);
//     }
// }
