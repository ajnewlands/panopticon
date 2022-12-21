use anyhow::Result;
use log::*;

use windows::{
    core::*, Win32::Media::Audio::Endpoints::*, Win32::Media::Audio::*,
    Win32::System::Com::StructuredStorage::*, Win32::System::Com::*, Win32::UI::Accessibility::*,
    Win32::UI::Shell::PropertiesSystem::*, Win32::UI::WindowsAndMessaging::*,
};

use eframe::{
    egui,
    epaint::{CircleShape, Color32, PathShape, Pos2, Stroke},
};

static FRONT_LEFT: usize = 0;
static FRONT_RIGHT: usize = 1;
static FRONT: usize = 2;
static REAR_LEFT: usize = 4;
static REAR_RIGHT: usize = 5;
static LEFT: usize = 6;
static RIGHT: usize = 7;

static WINDOW_SIZE: f32 = 320.;
static INNER_RADIUS_FACTOR: f32 = 0.4;
static OUTER_RADIUS: f32 = WINDOW_SIZE / 2. - 20.;

fn arc_points(range: std::ops::Range<i32>) -> Vec<Pos2> {
    let center = Pos2 {
        x: WINDOW_SIZE / 2.,
        y: WINDOW_SIZE / 2.,
    };
    // outer arc
    let mut points: Vec<Pos2> = range
        .clone()
        .map(|theta| Pos2 {
            x: (theta as f32 * std::f32::consts::PI / 180.).cos() * OUTER_RADIUS + center.x,
            y: (theta as f32 * std::f32::consts::PI / 180.).sin() * OUTER_RADIUS + center.y,
        })
        .collect();

    // inner arc
    let mut inner_points: Vec<Pos2> = range
        .rev()
        .map(|theta| Pos2 {
            x: (theta as f32 * std::f32::consts::PI / 180.).cos()
                * OUTER_RADIUS
                * INNER_RADIUS_FACTOR
                + center.x,
            y: (theta as f32 * std::f32::consts::PI / 180.).sin()
                * OUTER_RADIUS
                * INNER_RADIUS_FACTOR
                + center.y,
        })
        .collect();

    points.append(&mut inner_points);

    points
}

fn get_audio_interface() -> Result<()> {
    unsafe {
        info!("Initializing COM");
        let res = CoInitialize(None);
        if res.is_err() {
            let error = format!("Failed to init '{:?}'", res);
            MessageBoxA(
                None,
                Some(PCSTR::from_raw(error.as_ptr())),
                s!("Error"),
                MB_OK,
            );
        }

        info!("Creating instance");
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        info!("Getting default endpoint");
        let endpoint = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        info!("Getting endpoint id");

        let meter: IAudioMeterInformation = endpoint.Activate(CLSCTX_ALL, None)?;

        info!("Got audio meter");

        let channel_count = meter.GetMeteringChannelCount()?;
        if channel_count != 8 {
            let error = format!(
                "Expected 8 channels for 7.1 audio, found only {}",
                channel_count
            );
            MessageBoxA(
                None,
                Some(PCSTR::from_raw(error.as_ptr())),
                s!("Error"),
                MB_OK,
            );
            std::process::exit(1);
        }

        let front_points = arc_points(250..291);
        let front_right_points = arc_points(290..341);
        let right_points = arc_points(340..391);
        let rear_right_points = arc_points(30..91);
        let rear_left_points = arc_points(90..151);
        let left_points = arc_points(150..201);
        let front_left_points = arc_points(200..251);

        let options = eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(WINDOW_SIZE, WINDOW_SIZE)),
            ..Default::default()
        };
        eframe::run_native(
            "Panopticon",
            options,
            Box::new(|_cc| {
                Box::new(PanApp {
                    front_points,
                    front_right_points,
                    right_points,
                    rear_right_points,
                    rear_left_points,
                    left_points,
                    front_left_points,
                    meter,
                })
            }),
        );
    }

    Ok(())
}

struct PanApp {
    front_points: Vec<Pos2>,
    front_right_points: Vec<Pos2>,
    right_points: Vec<Pos2>,
    rear_right_points: Vec<Pos2>,
    rear_left_points: Vec<Pos2>,
    left_points: Vec<Pos2>,
    front_left_points: Vec<Pos2>,
    meter: IAudioMeterInformation,
}

impl eframe::App for PanApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        static mut PEAK_VALUES: [f32; 8] = [0.; 8];
        egui::CentralPanel::default().show(ctx, |ui| {
            let painter = ui.painter();

            unsafe {
                self.meter.GetChannelsPeakValues(&mut PEAK_VALUES).unwrap();

                for (shape, meter) in [
                    (&self.front_points, PEAK_VALUES[FRONT]),
                    (&self.front_right_points, PEAK_VALUES[FRONT_RIGHT]),
                    (&self.right_points, PEAK_VALUES[RIGHT]),
                    (&self.rear_right_points, PEAK_VALUES[REAR_RIGHT]),
                    (&self.rear_left_points, PEAK_VALUES[REAR_LEFT]),
                    (&self.left_points, PEAK_VALUES[LEFT]),
                    (&self.front_left_points, PEAK_VALUES[FRONT_LEFT]),
                ] {
                    painter.add(PathShape {
                        points: shape.clone(),
                        closed: true,
                        fill: Color32::from_rgba_premultiplied((meter * 255.) as u8, 0, 0, 255),
                        stroke: Stroke {
                            width: 1.,
                            color: Color32::BLACK,
                        },
                    });
                }
            }

            //Concentric rings
            for factor in [1., 0.8, 0.6, 0.4, 0.2] {
                painter.add(CircleShape {
                    radius: OUTER_RADIUS * INNER_RADIUS_FACTOR * factor,
                    fill: Color32::BLACK,
                    stroke: Stroke {
                        width: 1.,
                        color: Color32::GREEN,
                    },
                    center: Pos2 {
                        x: WINDOW_SIZE / 2.,
                        y: WINDOW_SIZE / 2.,
                    },
                });
            }

            // Horitontal Radar axis
            painter.add(PathShape {
                points: vec![
                    Pos2 {
                        x: WINDOW_SIZE / 2. - (OUTER_RADIUS * INNER_RADIUS_FACTOR),
                        y: WINDOW_SIZE / 2.,
                    },
                    Pos2 {
                        x: WINDOW_SIZE / 2. + (OUTER_RADIUS * INNER_RADIUS_FACTOR),
                        y: WINDOW_SIZE / 2.,
                    },
                ],
                stroke: Stroke {
                    width: 1.,
                    color: Color32::GREEN,
                },
                closed: false,
                fill: Color32::TRANSPARENT,
            });

            // Verical Radar axis
            painter.add(PathShape {
                points: vec![
                    Pos2 {
                        x: WINDOW_SIZE / 2.,
                        y: WINDOW_SIZE / 2. - (OUTER_RADIUS * INNER_RADIUS_FACTOR),
                    },
                    Pos2 {
                        x: WINDOW_SIZE / 2.,
                        y: WINDOW_SIZE / 2. + (OUTER_RADIUS * INNER_RADIUS_FACTOR),
                    },
                ],
                stroke: Stroke {
                    width: 1.,
                    color: Color32::GREEN,
                },
                closed: false,
                fill: Color32::TRANSPARENT,
            });

            // draw radar sweep
            let angle = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                / 10
                % 360;

            // Sweeping hand
            let sweep = Pos2 {
                x: (angle as f32 * std::f32::consts::PI / 180.).cos()
                    * OUTER_RADIUS
                    * INNER_RADIUS_FACTOR
                    + WINDOW_SIZE / 2.,
                y: (angle as f32 * std::f32::consts::PI / 180.).sin()
                    * OUTER_RADIUS
                    * INNER_RADIUS_FACTOR
                    + WINDOW_SIZE / 2.,
            };
            painter.add(PathShape {
                points: vec![
                    Pos2 {
                        x: WINDOW_SIZE / 2.,
                        y: WINDOW_SIZE / 2.,
                    },
                    sweep,
                ],
                stroke: Stroke {
                    width: 2.,
                    color: Color32::LIGHT_GREEN,
                },
                closed: false,
                fill: Color32::TRANSPARENT,
            });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    if let Err(e) = get_audio_interface() {
        error!("{:?}", e);
        std::process::exit(1);
    }
}
