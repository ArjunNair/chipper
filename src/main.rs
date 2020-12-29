//extern crate egui_sdl2_gl;
#![windows_subsystem = "windows"]

extern crate gl;

use std::{fs, io, path::PathBuf, collections::HashMap};
use sdl2::event::Event;
use sdl2::keyboard::Keycode::*;
use std::time::{Duration, Instant};
use chip8::Chip8;
mod chip8;
use egui::{Image, Rect, Pos2, Srgba, color, combo_box_with_label, vec2};

// Helper function to get all valid Chip8 ROM Files in the "roms"
// directory. The dictionary maps a filename to a file path.
fn get_roms(dir: &str) -> io::Result<HashMap<String, String>> {
    let mut files : HashMap<String, String> = HashMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let data = entry.metadata()?;
        let path = entry.path();
        let file_name = entry.file_name().into_string().unwrap();
        if data.is_file() {
            if let Some(ex) = path.extension() {
                if ex == "ch8" {
                    //println!("Found rom: {}", file_name);
                    files.insert(file_name, path.display().to_string());                
                }
            }
        }
    }
    Ok(files)
}

// Helper function to convert a SDL2 keycode to a Chip8 key.
fn keycode_to_chip8_key(keycode: &sdl2::keyboard::Keycode) -> u8{
    let key : u8;
    match keycode {
        Num0 => key = 0,
        Num1 => key = 1,
        Num2 => key = 2,
        Num3 => key = 3,
        Num4 => key = 4,
        Num5 => key = 5,
        Num6 => key = 6,
        Num7 => key = 7,
        Num8 => key = 8,
        Num9 => key = 9,
        A => key = 0xa,
        B => key = 0xb,
        C => key = 0xc,
        D => key = 0xd,
        E => key = 0xe,
        F => key = 0xf,
        _ => key = 0xff
    };
    key
}

pub fn main() {
    const CHIP8_DISPLAY_WIDTH: u32 = 64;
    const CHIP8_DISPLAY_HEIGHT: u32 = 32;
    const DISPLAY_SCALE: u32 = 8;
    const WINDOW_WIDTH: u32 = CHIP8_DISPLAY_WIDTH * DISPLAY_SCALE + 8;
    const WINDOW_HEIGHT: u32 = 420;

    let rom_path = PathBuf::from("./roms");
    let rom_files =  get_roms(&rom_path.display().to_string()).unwrap();
    let mut selected_rom = "ChipperBoot.ch8";

    //for (filename, _path) in &rom_files {
    //    selected_rom = filename;
    //    break;
    //}
    
    let mut chip8 = Chip8::new();
    chip8.boot_rom(rom_files.get(selected_rom).expect("No rom files to load!")).expect("Failed to load rom!");

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Chipper - Chip8 Emulator in Rust", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        //.resizable()
        .opengl()
        .build()
        .unwrap();

    let _ctx = window.gl_create_context().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    //Egui related stuff
    let mut painter = egui_sdl::Painter::new(&video_subsystem, WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut egui_ctx = egui::CtxRef::default();
    let pixels_per_point = 96f32 / video_subsystem.display_dpi(0).unwrap().0;
    let (width, height) = window.size();
    let mut raw_input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::new(0f32, 0f32), vec2(width as f32, height as f32) / pixels_per_point)),
        pixels_per_point: Some(pixels_per_point),
        ..Default::default()
    };
    let mut clipboard = egui_sdl::init_clipboard();
    //End of egui related stuff

    let start_time = Instant::now();
    let mut srgba: Vec<Srgba> = Vec::new();

    let chip8_display = chip8.get_display_data();
    for y in 0..CHIP8_DISPLAY_HEIGHT as usize {
        for x in 0..CHIP8_DISPLAY_WIDTH as usize{
            let pixel  = chip8_display[y * (CHIP8_DISPLAY_WIDTH as usize) + x];
            let c = if pixel > 0 {color::LIGHT_GRAY} else {color::BLACK};
            srgba.push(c);
        }
    }
    let chip8_tex_id = painter.new_user_texture((CHIP8_DISPLAY_WIDTH as usize, CHIP8_DISPLAY_HEIGHT as usize), srgba.as_slice(), false);
    let bg_color = color::srgba(128, 128, 128, 0);
    let mut use_vy_for_shift_operations = false;
    let mut increment_i_on_ld_operations = false;
    let mut frame_count= 0;
    let mut avg_frame_time = 0u128;
    let mut fps = 0u128;
    let mut frame_time_accum = 0u128;
    let mut is_paused = false;

    //The main loop. 
    //Processes events, runs emulation steps, updates display
    'running: loop {
        let frame_time = Instant::now();  
        raw_input.time = Some(start_time.elapsed().as_nanos() as f64 * 1e-9);
        egui_ctx.begin_frame(raw_input.take());
        
        let mut srgba: Vec<Srgba> = Vec::new();
    
        //The chip8 display will be blit to this texture every frame.
        let chip8_display = chip8.get_display_data();
        for y in 0..CHIP8_DISPLAY_HEIGHT as usize {
            for x in 0..CHIP8_DISPLAY_WIDTH as usize{
                let pixel  = chip8_display[y * (CHIP8_DISPLAY_WIDTH as usize) + x];
                let c = if pixel > 0 {color::BLACK} else {color::LIGHT_GRAY};
                srgba.push(c);
            }
        }

        painter.update_user_texture_data(chip8_tex_id, &srgba);
               
        &egui::Window::new("Chipper")
            .fixed_pos(Pos2::new(0f32,0f32))
            //.default_size(vec2(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32))
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .show(&mut egui_ctx, |ui| {
                if !is_paused {
                    ui.label(format!("FPS: {} ({} ms/frame)", fps, avg_frame_time));
                }
                else {
                    ui.label(format!("PAUSED"));
                }
               
                ui.add(Image::new(chip8_tex_id, vec2((CHIP8_DISPLAY_WIDTH * DISPLAY_SCALE) as f32, (CHIP8_DISPLAY_HEIGHT * DISPLAY_SCALE) as f32)));
                ui.label("");
                
                combo_box_with_label(ui, "ROM files", selected_rom, |ui| { 
                    //Doesn't work ATM 
                    for (f, _p) in &rom_files {
                        if ui.selectable_value(&mut selected_rom, f, f).clicked {
                             chip8.boot_rom(rom_files.get(selected_rom).expect("No rom files to load!")).expect("Failed to load rom!");
                        };
                        /*if ui.button(f).clicked {
                            selected_rom = f;
                            chip8.boot_rom(rom_files.get(selected_rom).expect("No rom files to load!")).expect("Failed to load rom!");
                        };*/
                    }
                });
                //There is probably a better way to add line breaks in egui....
                ui.label("");
                if ui.checkbox(&mut use_vy_for_shift_operations, "Use Vy for shift operations").clicked {
                    chip8.shift_using_vy = use_vy_for_shift_operations;
                };  
                if ui.checkbox(&mut increment_i_on_ld_operations, "Increment I on  LD Vx operations").clicked {
                    chip8.increment_i_on_ld = increment_i_on_ld_operations;
                };
                ui.label("");
                ui.label("ESC = Pause/Resume.  F2 = Reset.");
                
        });
       
        let (_output, paint_cmds) = egui_ctx.end_frame();
        let paint_jobs = egui_ctx.tesselate(paint_cmds);
        painter.paint_jobs(bg_color, paint_jobs, &egui_ctx.texture(), pixels_per_point);
        
        window.gl_swap_window();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(t), ..} =>  {
                    match t {
                        Num0 | Num1 | Num2 | Num3 | Num4 | Num5 | Num6 | Num7 | Num8| Num9 => {
                            chip8.set_key_pressed(keycode_to_chip8_key(&t));
                        },
                        A | B | C| D | E | F  => {
                            chip8.set_key_pressed(keycode_to_chip8_key(&t));
                        },
                        _ => chip8.set_key_pressed(0xff)
                    }
                },
                Event::KeyUp { keycode: Some(t), ..} => {
                    match t {
                        Num0 | Num1 | Num2 | Num3 | Num4 | Num5 | Num6 | Num7 | Num8| Num9 => {
                            chip8.set_key_pressed(0xff);
                        },
                        A | B | C| D | E | F  => {
                            chip8.set_key_pressed(0xff);
                        },
                        Escape => {
                            is_paused = !is_paused;
                        },
                        F2 => {
                            chip8.boot_rom(rom_files.get(selected_rom).expect("No rom files to load!")).expect("Failed to load rom!");
                        }
                        _ => ()
                    }
                },
                _ => {
                    egui_sdl::input_to_egui(event, clipboard.as_mut(), &mut raw_input);
                }
            }
        }

        if !is_paused {
            for _ in 0 .. 10 {
                chip8.step();
            }
        }
        
        let elapsed_frame_time =  frame_time.elapsed();
        let frame_time_in_ms = elapsed_frame_time.as_millis();
        //let frame_time_in_ns =  elapsed_frame_time.as_nanos();

        //Try to maintain ~60FPS. This isn't the best way, but it's fine for
        //for now.
        if frame_time_in_ms < 16 {
            //std::thread::sleep(Duration::from_nanos((16000000 - frame_time_in_ns) as u64));  
            std::thread::sleep(Duration::from_millis((16 - frame_time_in_ms) as u64));
        }
        let frame_time_in_ms = frame_time.elapsed().as_millis();
        frame_time_accum += frame_time_in_ms;
 
        frame_count += 1;

        if frame_count >= 10 {
            avg_frame_time = frame_time_accum/frame_count;
            if avg_frame_time > 0 {
                fps = 1000/avg_frame_time;
            }
            frame_time_accum = 0u128;
            frame_count = 0;
        }
        chip8.update_timers();
    }
    painter.cleanup();
}