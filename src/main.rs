mod nes;
use std::time::{Instant, Duration, self};

use iced::{self, keyboard, subscription};

use eframe::egui::{self, CollapsingHeader};
use eframe::epaint::ColorImage;
use egui_extras::{self, RetainedImage};
use image::{DynamicImage, EncodableLayout};

use crate::nes::Nes;

use iced::{executor, Subscription};
use iced::{Application, Command, Element, Settings, Theme};
use iced::widget;

pub fn main() { // -> iced::Result {
    // let mut nes = Nes::new();
    // nes.load_rom(String::from("donkey_kong.nes"));
    // nes.ppu.borrow().render_chr();
    Hello::run(Settings::default());
    // let native_options = eframe::NativeOptions::default();
    // eframe::run_native("My egui App",
    //     native_options,
    //     Box::new(|cc| {
    //         Box::<MyEguiApp>::default()
    //     }),
    // );

}


struct MyEguiApp {
    nes: Nes,

    chr_data: Option<RetainedImage>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let mut app = Self::default();

        app.nes.load_rom(String::from("donkey_kong.nes"));
        app
    }
}

impl Default for MyEguiApp {
    fn default() -> Self {
        let mut nes = Nes::default();
        nes.load_rom(String::from("donkey_kong.nes"));
        // let chr_image = nes.ppu.borrow().render_chr();
        // let chr_image = DynamicImage::ImageLuma8(chr_image).into_rgba8();
        // let chr_data = egui::ColorImage::from_rgba_unmultiplied(
        //     [chr_image.width() as _, chr_image.height() as _],
        //     chr_image.as_flat_samples().as_slice()
        // );
        // let chr_data = RetainedImage::from_color_image("chr_data", chr_data);
        Self { nes, chr_data: None}

    }
}

impl eframe::App for MyEguiApp {
   fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
       egui::CentralPanel::default().show(ctx, |ui| {
           ui.heading("Hello World!");
           ui.label("Game field");
           ui.collapsing("Debug Info", |ui| {
            ui.label("CHR Data");
            let chr_image = self.nes.ppu.borrow().render_chr();
            let chr_image = DynamicImage::ImageLuma8(chr_image).into_rgba8();
            let chr_data = egui::ColorImage::from_rgba_unmultiplied(
                [chr_image.width() as _, chr_image.height() as _],
                chr_image.as_flat_samples().as_slice()
            );
            self.chr_data = Some(RetainedImage::from_color_image("chr_data", chr_data));
            self.chr_data.as_mut().unwrap().show(ui);
           });
       });
   }
}

struct Hello {
    nes: Nes,

    // state
    frame_rate: f64,
    controller_state: IcedControllerState,

    // cached images
    chr_image: Option<image::RgbaImage>,
    frame: image::RgbaImage
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    RefreshChrPressed,
    Tick(Instant),
    Event(iced::Event)
}

impl Application for Hello {
    type Executor = executor::Default;
    type Flags = ();
    type Message = AppMessage;
    type Theme = Theme;

    fn new(_flags: ()) -> (Hello, Command<Self::Message>) {
        let mut nes = Nes::default();
        nes.load_rom(String::from("donkey_kong.nes"));
        // nes.load_rom(String::from("nes-test-roms/full_palette/full_palette.nes"));
        // nes.load_rom(String::from("nes-test-roms/ppu_vbl_nmi/rom_singles/01-vbl_basics.nes"));
        (Hello {
            nes,
            chr_image: None,
            frame_rate: 60.0,
            frame: image::RgbaImage::new(256,240),
            controller_state: IcedControllerState::default()
        }, Command::none())
    }

    fn title(&self) -> String {
        String::from("A cool application")
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if true {//self.is_playing {
            Subscription::batch([
            iced::time::every(Duration::from_millis(1000 / self.frame_rate as u64))
                .map(Self::Message::Tick),
            subscription::events().map(Self::Message::Event)])
            //iced::event::listen().map(Self.Message::Event)
        } else {
            Subscription::none()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Self::Message::RefreshChrPressed => {
                let chr_image = self.nes.ppu.borrow().render_chr();
                self.chr_image = Some(DynamicImage::ImageLuma8(chr_image).into_rgba8());
                self.nes.ppu.borrow().print_nametable();
            },
            Self::Message::Tick(instant) => {
                println!("Frame update");
                let t = time::SystemTime::now();
                self.nes.inputs.borrow_mut().set_controller1_state(self.controller_state.state);
                self.frame = DynamicImage::ImageRgb8(self.nes.run_frame()).into_rgba8();
                let d = t.elapsed();
                println!("Took {}s",d.unwrap().as_millis());
            }
            Self::Message::Event(event) => {
                match event {
                    iced::Event::Keyboard(key_event) => {
                        self.controller_state.onEvent(key_event);
                    },
                    _ => {}
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        // "Hello, world!".into();
        // let chr_image = self.nes.ppu.borrow().render_chr();
        // let chr_image = DynamicImage::ImageLuma8(chr_image).into_rgba8().as_bytes().to_owned();
        let chr_image = if let Some(image) = self.chr_image.clone() {
            widget::image::Handle::from_pixels(image.width(),image.height(),image.as_bytes().to_owned())
        } else {
            widget::image::Handle::from_pixels(200, 200, [0u8;200*200*4])
        };
        let frame = self.frame.clone();
        let frame = widget::image::Handle::from_pixels(frame.width(),frame.height(),frame.as_bytes().to_owned());
        // widget::image::Handle::from_pixels(144,171,Some(chr_image);
        let content = iced::widget::column![
            widget::text(String::from("NES Screen")).size(30).width(iced::Length::Fill),
            widget::image::viewer(frame)
                .max_scale(1.0)
                .min_scale(2.0)
                .width(iced::Length::Fill),
            widget::text(String::from("CHR Data")).size(30).width(iced::Length::Fill),
            widget::image::viewer(chr_image)
                .max_scale(2.0)
                .min_scale(2.0)
                .width(iced::Length::Fill),
            widget::button("Refresh").on_press(Self::Message::RefreshChrPressed)
        ];
        iced::widget::container(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}


#[derive(Default)]
struct IcedControllerState {
    state: nes::input::ControllerState
}

impl IcedControllerState {
    fn onEvent(&mut self, event: iced::keyboard::Event) {
                println!("Got event!");
        if let Some((key_code, active)) = match event {
            keyboard::Event::KeyPressed{key_code, modifiers: _} => Some((key_code, true)),
            keyboard::Event::KeyReleased { key_code, modifiers: _ } => Some((key_code, false)),
            _ => None
        } {
            print!("Got key {:?}",key_code);
            match key_code {
                keyboard::KeyCode::Up => self.state.up = active,
                keyboard::KeyCode::Down => self.state.down = active,
                keyboard::KeyCode::Left => self.state.left = active,
                keyboard::KeyCode::Right => self.state.right = active,
                keyboard::KeyCode::Enter => self.state.start = active,
                keyboard::KeyCode::RShift => self.state.select = active,
                keyboard::KeyCode::X => self.state.a = active,
                keyboard::KeyCode::Z => self.state.b = active,
                _ => {}
            }
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
