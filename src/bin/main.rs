#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]


use embassy_executor::Spawner;

use embassy_sync::channel::Channel;
use embassy_time::Instant;
use embassy_time::{Duration, Timer};
use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::iso_8859_13::FONT_9X18_BOLD;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::pixelcolor::Rgb565;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::InputConfig;
use esp_hal::peripherals::{GPIO0, GPIO5, RMT};
use esp_hal::peripherals::GPIO1;
use esp_hal::peripherals::GPIO8;
use esp_hal::timer::timg::TimerGroup;
use core::fmt::Write;
use smart_leds_trait::SmartLedsWrite;
use smart_leds_trait::RGB8;
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;
use esp_hal::gpio::Level;
use profont::PROFONT_24_POINT;
use esp_hal::gpio::{OutputConfig};
use esp_hal::time::Rate;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb666,
    prelude::*,
    text::Text,
};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use u8g2_fonts::{fonts, U8g2TextStyle};
use core::cell::RefCell;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]



static POS: Mutex<CriticalSectionRawMutex, RefCell<i32>> =
    Mutex::new(RefCell::new(0));


static MENU: Mutex<CriticalSectionRawMutex, RefCell<APPMODE>> =
    Mutex::new(RefCell::new(APPMODE::MAIN));

static INMENU: Mutex<CriticalSectionRawMutex, RefCell<Option<INMENU>>> =
    Mutex::new(RefCell::new(None));

static INCOLOR: Mutex<CriticalSectionRawMutex, RefCell<Option<INCOLOR>>> =
    Mutex::new(RefCell::new(None));
static PAUSE: Mutex<CriticalSectionRawMutex, RefCell<bool>> =
    Mutex::new(RefCell::new(true));
    
static ROTARY_CH: Channel<CriticalSectionRawMutex, bool, 8> = Channel::new();

static RGB_DATA: Mutex<CriticalSectionRawMutex, RefCell<(u8,u8,u8)>> =
    Mutex::new(RefCell::new((0,0,0)));

static RGB_CH: Channel<CriticalSectionRawMutex, CHANGELED, 8> = Channel::new();
    
static BRIGHTNESS: Mutex<CriticalSectionRawMutex, RefCell<u8>> =
    Mutex::new(RefCell::new(0));



#[derive(Clone, Copy,Debug,PartialEq)]
enum INCOLOR {
    RED,
    BLUE,
    GREEN,
}
#[derive(Clone, Copy,Debug,PartialEq)]
enum INMENU {
    BRIGHTNESS,
    COLOR,
}
#[derive(Clone, Copy,Debug,PartialEq)]
enum CHANGELED {
    BRIGHTNESS,
    COLOR,
    MODE
}
#[derive(Clone, Copy,Debug,PartialEq)]
enum APPMODE {
    MAIN,
    BRIGHTNESS,
    COLOR,
}
impl APPMODE {
    fn next(&mut self) -> Self {
        match self {
            APPMODE::MAIN => APPMODE::BRIGHTNESS,
            APPMODE::BRIGHTNESS => APPMODE::COLOR,
            APPMODE::COLOR => APPMODE::MAIN,
        }
    }

    fn prev(&mut self) -> Self {
        match self {
            APPMODE::MAIN => APPMODE::COLOR,
            APPMODE::BRIGHTNESS => APPMODE::MAIN,
            APPMODE::COLOR => APPMODE::BRIGHTNESS,
        }
    }
}

#[embassy_executor::task]
async fn led_task(
    gpio5: GPIO5<'static>,
    rmt: RMT<'static>
    
) {
    let mut buffer = esp_hal_smartled::smart_led_buffer!(17);
    let rmt = esp_hal::rmt::Rmt::new(rmt, esp_hal::time::Rate::from_mhz(80)).unwrap();
    let channel = rmt.channel0;
    
    let mut led= esp_hal_smartled::SmartLedsAdapter::new(channel, gpio5, &mut buffer);
    let mut r = 0_u8;
    let mut g = 0_u8;
    let mut b = 0_u8;
    let mut flag_bri = BRIGHTNESS.lock(|d| *d.borrow());
    // let mut rgb_co = [RGB8::new(255, 255, 255); 17];
    loop{
        log::info!("LED TASK");
        let sig = RGB_CH.receive().await;
        match sig {
            CHANGELED::BRIGHTNESS=>{
                let curr = BRIGHTNESS.lock(|d| *d.borrow());

                if curr > flag_bri {
                    let diff = curr - flag_bri;
                    r = r.saturating_add(diff);
                    g = g.saturating_add(diff);
                    b = b.saturating_add(diff);
                } else if curr < flag_bri {
                    let diff = flag_bri - curr;
                    r = r.saturating_sub(diff);
                    g = g.saturating_sub(diff);
                    b = b.saturating_sub(diff);
                }
                log::info!(
                    "prev={}, curr={}, rgb=({}, {}, {})",
                    flag_bri,
                    curr,
                    r,
                    g,
                    b
                );
                flag_bri = curr;
                let rgb_co = [RGB8::new(r, g, b); 17];
                led.write(rgb_co.into_iter()).unwrap();
            },
            CHANGELED::COLOR=>{

            },
            CHANGELED::MODE=>{

            }
        }
        
        Timer::after(Duration::from_millis(1)).await;

    }
}

#[embassy_executor::task]
async fn stick_task(
    gpio0: GPIO0<'static>, // CLK
    gpio1: GPIO1<'static>, // DT
    gpio8: GPIO8<'static>, // SW
) {
    use embassy_time::{Duration, Timer};
    use esp_hal::gpio::InputConfig;

    let input_cfg = InputConfig::default().with_pull(esp_hal::gpio::Pull::Up);

    let clk = esp_hal::gpio::Input::new(gpio0, input_cfg);
    let dt  = esp_hal::gpio::Input::new(gpio1, input_cfg);
    let sw  = esp_hal::gpio::Input::new(gpio8, input_cfg);

    // 현재 엔코더 2비트 상태
    let mut prev_state: u8 = ((clk.is_high() as u8) << 1) | (dt.is_high() as u8);
    let mut acc: i8 = 0;

    // 0~255 제어값
    let mut value: u8 = 128;

    // 버튼용
    let mut prev_sw = sw.is_high();
    let mut btn_block = false;

    loop {
        // -------- rotary --------
        let clk_now = clk.is_high();
        let dt_now  = dt.is_high();
        let state = ((clk_now as u8) << 1) | (dt_now as u8);

        match (prev_state, state) {
            // CW
            (0b00, 0b01) | (0b01, 0b11) | (0b11, 0b10) | (0b10, 0b00) => {
                acc += 1;
            }
            // CCW
            (0b00, 0b10) | (0b10, 0b11) | (0b11, 0b01) | (0b01, 0b00) => {
                acc -= 1;
            }
            // bounce / invalid transition
            _ => {}
        }

        prev_state = state;

        // 보통 1 detent = 4 transition
        if acc >= 3 {
            acc = 0;
            value = value.saturating_add(1);
            log::info!("CW  value={}", value);
            let _ =ROTARY_CH.try_send(true);
                INMENU.lock(|i_m|{
                    if let Some(in_m)=*i_m.borrow(){
                        match in_m {
                            INMENU::BRIGHTNESS=>{
                                BRIGHTNESS.lock(|b|{
                                    let mut b = b.borrow_mut();
                                    *b = b.saturating_add(1);
                                    let _ =RGB_CH.try_send(CHANGELED::BRIGHTNESS);
                                    
                                });
                            },
                            INMENU::COLOR=>{

                            }
                        }
                    }else{
                        let mut pos_t=0;
                        MENU.lock(|m| {
                            let mut mode = m.borrow_mut();
                            *mode = mode.next();
                            log::info!("MENU={:?}", mode);
                        });
                        POS.lock(|p| {
                            *p.borrow_mut() -= 1;
                            pos_t=*p.borrow();
                        });
                        // POS.fetch_add(1, Ordering::Relaxed);
                        log::info!("CCW pos={}", pos_t);
                    }
                });
        } else if acc <= -3 {
            acc = 0;
            value = value.saturating_sub(1);
            log::info!("CCW value={}", value);
            let _ =ROTARY_CH.try_send(true);
                INMENU.lock(|i_m|{
                    if let Some(in_m)=*i_m.borrow(){
                        match in_m {
                            INMENU::BRIGHTNESS=>{
                                BRIGHTNESS.lock(|b|{
                                    let mut b = b.borrow_mut();
                                    *b = b.saturating_sub(1);
                                    let _ =RGB_CH.try_send(CHANGELED::BRIGHTNESS);
                                });
                            },
                            INMENU::COLOR=>{
                                let in_c =INCOLOR.lock(|c|*c.borrow());
                                if let Some(inc)=in_c{
                                    match inc {
                                        INCOLOR::RED=>{
                                            log::info!("REC CAL");
                                        },
                                        INCOLOR::GREEN=>{
                                            log::info!("GREEN CAL");
                                        },
                                        INCOLOR::BLUE=>{
                                            log::info!("BLUE CAL");
                                        },
                                    }
                                }
                            }
                        }
                    }else{
                        let mut pos_t=0;
                        MENU.lock(|m| {
                            let mut mode = m.borrow_mut();
                            *mode = mode.prev();
                            // *m=*m..prev();
                            log::info!("MENU={:?}", mode);
                        });
                        POS.lock(|p| {
                            *p.borrow_mut() += 1;
                            pos_t=*p.borrow();
                            // *p += 1;
                            // pos_t=*p;
                        });
                        // pos += 1;
                        log::info!("CW pos={}", pos_t);
                    }
                });
        }

        // -------- button --------
        let sw_now = sw.is_high();

        // pull-up 기준: 안누름=HIGH, 누름=LOW
        if prev_sw && !sw_now && !btn_block {
            btn_block = true;
            log::info!("BUTTON value={}", value);
            let _ =ROTARY_CH.try_send(true);
            MENU.lock(|m| {
                let mut mode = m.borrow_mut();
                match *mode {
                    APPMODE::BRIGHTNESS=>{
                        INMENU.lock(|i_m|{
                            let mut m = i_m.borrow_mut();
                            match *m {
                                None=>{
                                    *m=Some(INMENU::BRIGHTNESS);
                                },
                                Some(_)=>{
                                    *m=None;
                                }
                            }
                            
                        });
                    },
                    APPMODE::COLOR=>{
                        INMENU.lock(|i_m|{
                            let mut m = i_m.borrow_mut();
                            match *m {
                                None=>{
                                    *m=Some(INMENU::COLOR);
                                    INCOLOR.lock(|co|{
                                        *co.borrow_mut()=Some(INCOLOR::RED);
                                        log::info!("CHANGE RED");
                                    });
                                },
                                Some(_)=>{
                                    INCOLOR.lock(|co|{
                                        let mut co = co.borrow_mut();
                                        if let Some(i_c)=*co{
                                            match i_c {
                                                INCOLOR::RED=>{
                                                    log::info!("CHANGE GREEN");
                                                    *co=Some(INCOLOR::GREEN);
                                                },
                                                INCOLOR::GREEN=>{
                                                    log::info!("CHANGE BLUE");
                                                    *co=Some(INCOLOR::BLUE);
                                                },
                                                INCOLOR::BLUE=>{
                                                    log::info!("CHANGE NONE");
                                                    *co=None;
                                                    *m=None;
                                                },
                                            }
                                            
                                        }
                                        
                                    });
                                    
                                }
                            }
                            
                        });
                    },
                    _=>{
                        INMENU.lock(|i_m|{
                            let mut m = i_m.borrow_mut();
                            *m=None;
                        });
                    }
                }
            });
            ROTARY_CH.try_send(true);

            log::info!("BUTTON");
            
        }

        if !prev_sw && sw_now {
            btn_block = false;
        }

        prev_sw = sw_now;

        Timer::after(Duration::from_micros(100)).await;
    }
}
#[embassy_executor::task]
async fn pause_task() {
    let mut start: Option<Instant> = None;

    loop {
        if let Ok(ev) = ROTARY_CH.try_receive() {
            match ev {
                true => {
                    start = Some(Instant::now());
                    PAUSE.lock(|p| {
                        *p.borrow_mut() = true;
                    });
                    log::info!("PAUSE");
                }
                false => {
                    start = None;
                    PAUSE.lock(|p| {
                        *p.borrow_mut() = false;
                    });
                    log::info!("false");
                }
            }
        }

        if let Some(s) = start {
            if s.elapsed() >= Duration::from_secs(5) {
                PAUSE.lock(|p| {
                    *p.borrow_mut() = false;
                });
                
                start = None;

                log::info!("timeout -> false");
            }
        }

        Timer::after(Duration::from_millis(10)).await;
    }
}
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    
    
    let sck  = peripherals.GPIO4;
    let mosi = peripherals.GPIO6;
    let cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());


    
    // let mut prev_clk = clk.is_high();
    // let mut prev_sw = sw.is_high();
    // let mut pos: i32 = 0;

    let spi_config = esp_hal::spi::master::Config::default().with_frequency(Rate::from_mhz(40)).with_mode(esp_hal::spi::Mode::_0);
    let mut spi = Spi::new(peripherals.SPI2, spi_config).unwrap()
        .with_sck(sck)
        // .with_cs(cs)
        .with_mosi(mosi);
    let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut delay = esp_hal::delay::Delay::new();
    let mut tft_buf = [0u8; 512];
    let di = mipidsi::interface::SpiInterface::new(spi_dev, dc, &mut tft_buf);
    let mut display= mipidsi::Builder::new(mipidsi::models::ST7796, di)
        .reset_pin(rst)
        .orientation(
            mipidsi::options::Orientation::new()
                .rotate(mipidsi::options::Rotation::Deg90)
                .flip_horizontal()
        )
        .color_order(mipidsi::options::ColorOrder::Bgr)
        .init(&mut delay).unwrap();
    // let di = mipidsi::interface::SpiInterface::new(spi, dc, &mut tft_buf);
    display.clear(Rgb565::BLACK.into()).unwrap();
    display.clear(Rgb565::RED.into()).unwrap();
    
    let text_style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
    let style = U8g2TextStyle::new(
        u8g2_fonts::fonts::u8g2_font_helvB18_tr,
        Rgb565::WHITE,
    );
    let rec=Text::new("HIIIIIIIIIIIII ", Point::new(20, 40), &style)
        .draw(&mut display)
        .unwrap();
    let rec=Text::new("Yes ~~~~~~~~~~", rec, &style)
        .draw(&mut display)
        .unwrap();
    let mut rec = Point::new(20, 80);
    for _ in 0..56{
        
        let res=Text::new("-", rec, &style)
            .draw(&mut display)
            .unwrap();
            rec=res;
    }
    

    
    let mut flag_bri = 255_u8;
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    // let data = [RGB8::new(8, 0, 0); 17];
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let mut menu_t = APPMODE::MAIN;
    let mut in_menu_t:Option<INMENU> = None;
    let mut in_color_t:Option<INCOLOR> = None;
    spawner.spawn(stick_task(
        peripherals.GPIO0,
        peripherals.GPIO1,
        peripherals.GPIO8,
    )).unwrap();
    spawner.spawn(pause_task()).unwrap();
    spawner.spawn(led_task(peripherals.GPIO5,peripherals.RMT)).unwrap();
    
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    // let mut led = esp_hal_smartled::SmartLedsAsdapter::new(led_pin);
    // TODO: Spawn some tasks
    // let _ = spawner;
    Text::new("HELLO MAIN", Point::new(150, 110), &style)
    .draw(&mut display)
    .unwrap();
    let mut t_rec = Point::new(20, 130);
    for _ in 0..32{
        
        let res=Text::new("=", t_rec, &style)
            .draw(&mut display)
            .unwrap();
            t_rec=res;
    }

    loop {
        INMENU.lock(|i_m|{
            let i_m = i_m.borrow();
            if in_menu_t != *i_m{
                embedded_graphics::primitives::Rectangle::new(Point::new(0, 140), Size::new(500, 180))
                .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                .draw(&mut display)
                .unwrap();
                // in_menu_t=*i_m;
            }
        });
        MENU.lock(|m| {
            let mut mode = m.borrow_mut();
            if menu_t !=*mode{
                in_menu_t=None;
                let txt = match *mode {
                    APPMODE::MAIN=>{
                        "MAIN"
                    },
                    APPMODE::BRIGHTNESS=>{
                        "BRIGHTNESS"
                    },
                    APPMODE::COLOR=>{
                        "COLOR"
                    }
                };

                
                embedded_graphics::primitives::Rectangle::new(Point::new(120, 80), Size::new(250, 40))
                    .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                    .draw(&mut display)
                    .unwrap();
                Text::new(txt, Point::new(150, 110), &style)
                .draw(&mut display)
                .unwrap();
                // match *mode {
                //     APPMODE::MAIN=>{
                        
                //     },
                //     APPMODE::BRIGHTNESS=>{
                        
                //     },
                //     APPMODE::COLOR=>{
                       
                //     }
                // };
                embedded_graphics::primitives::Rectangle::new(Point::new(0, 140), Size::new(500, 180))
                .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                .draw(&mut display)
                .unwrap();
            }
            // *mode = mode.prev();
            // *m=*m..prev();
            
            menu_t =*mode;
            
            // log::info!("MENU={:?}", mode);
        });
        PAUSE.lock(|l|{
            match *l.borrow() {
                true=>{
                    
                    match  menu_t{
                        APPMODE::MAIN=>{
                            main_body(&mut display);
                        },
                        APPMODE::BRIGHTNESS=>{
                            bri_body(&mut display,&mut in_menu_t, &mut flag_bri);
                        },
                        APPMODE::COLOR=>{
                            color_body(&mut display,&mut in_menu_t, &mut in_color_t);
                        }
                    }
                    
                    // menu_t
                },
                false=>{
                    MENU.lock(|m| {
                        let mut m  = m.borrow_mut();
                        *m=APPMODE::MAIN;
                        main_body(&mut display);
                    });
                    INMENU.lock(|i_m|{
                        let mut m = i_m.borrow_mut();
                        *m=None;
                    });
                    INCOLOR.lock(|co|{
                        *co.borrow_mut()=None;
                    });
                    in_menu_t=None;
                    in_color_t=None;
                    // menu_t=APPMODE::MAIN;
                }
            }
        });
        // log::info!("hello");
        // let red = [RGB8::new(254, 0, 0); 17];
        // led.write(red.into_iter()).unwrap();
        // Timer::after(Duration::from_millis(1000)).await;

        // let green = [RGB8::new(0, 254, 0); 17];
        // led.write(green.into_iter()).unwrap();
        // Timer::after(Duration::from_millis(1000)).await;

        // let blue = [RGB8::new(0, 0, 254); 17];
        // led.write(blue.into_iter()).unwrap();
        // Timer::after(Duration::from_millis(1000)).await;

        // let off = [RGB8::new(254, 254, 254); 17];
        // led.write(off.into_iter()).unwrap();
        // Timer::after(Duration::from_millis(1000)).await;

        // let off = [RGB8::new(32,32,32); 17];
        // led.write(off.into_iter()).unwrap();
        Timer::after(Duration::from_millis(10)).await;
        // Timer::after(Duration::from_secs(1)).await;
        
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}


fn main_body (
    display: &mut mipidsi::Display<mipidsi::interface::SpiInterface<'_, ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, embedded_hal_bus::spi::NoDelay>, Output<'_>>, mipidsi::models::ST7796, Output<'_>>
){
    let style = U8g2TextStyle::new(
        u8g2_fonts::fonts::u8g2_font_helvB18_tr,
        Rgb565::WHITE,
    );
    Text::new("BRIGHTNESS : ", Point::new(50, 180), &style)
        .draw(display)
        .unwrap();
    BRIGHTNESS.lock(|b|{
        let mut buf = heapless::String::<8>::new();
        let text = *b.borrow();
        write!(buf, "{}", text).unwrap();
        // write!(buf, "{}", val).unwrap();
        Text::new(&buf, Point::new(250, 180), &style)
            .draw(display)
            .unwrap();
    });
    Text::new("COLOR : ", Point::new(50, 230), &style)
        .draw(display)
        .unwrap();
    Text::new("MODE : ", Point::new(50, 280), &style)
        .draw(display)
        .unwrap();
    
}

fn bri_body (
    display: &mut mipidsi::Display<mipidsi::interface::SpiInterface<'_, ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, embedded_hal_bus::spi::NoDelay>, Output<'_>>, mipidsi::models::ST7796, Output<'_>>,
    in_menu_t:&mut Option<INMENU>,
    flag_bri:&mut u8,
){
    let style = U8g2TextStyle::new(
        u8g2_fonts::fonts::u8g2_font_helvB18_tr,
        Rgb565::WHITE,
    );
    INMENU.lock(|i_m|{
        let mut m = i_m.borrow_mut();
        if let Some(in_m)=*m{
            if in_m==INMENU::BRIGHTNESS{
                if let None=in_menu_t.as_ref(){
                    embedded_graphics::primitives::Rectangle::new(Point::new(0, 140), Size::new(500, 180))
                    .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                    .draw(display)
                    .unwrap();
                    Text::new("VALUE : ", Point::new(50, 180), &style)
                        .draw(display)
                        .unwrap();
                    let br_value = BRIGHTNESS.lock(|valuse|*valuse.borrow());

                    // let text = *value.borrow();
                    embedded_graphics::primitives::Rectangle::new(Point::new(200, 150), Size::new(100, 40))
                    .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                    .draw(display)
                    .unwrap();
                    let mut buf = heapless::String::<8>::new();
                    // let text = *value.borrow();
                    write!(buf, "{}", br_value).unwrap();
                    // write!(buf, "{}", val).unwrap();
                    Text::new(&buf, Point::new(250, 180), &style)
                        .draw(display)
                        .unwrap();
                    *in_menu_t=Some(INMENU::BRIGHTNESS);
                }
                
            }
            let br_value = BRIGHTNESS.lock(|valuse|*valuse.borrow());
            if *flag_bri != br_value{
                embedded_graphics::primitives::Rectangle::new(Point::new(200, 150), Size::new(100, 40))
                .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                .draw(display)
                .unwrap();
                let mut buf = heapless::String::<8>::new();
                write!(buf, "{}", br_value).unwrap();
                // write!(buf, "{}", val).unwrap();
                Text::new(&buf, Point::new(250, 180), &style)
                    .draw(display)
                    .unwrap();
                *flag_bri = br_value;
            }
            
            
        }else{
            Text::new("Click to adjust the brightness", Point::new(50, 180), &style)
                .draw(display)
                .unwrap();
            *in_menu_t=*m;
        }
        // *m=None;
    });
}

fn color_body (
    display: &mut mipidsi::Display<mipidsi::interface::SpiInterface<'_, ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, embedded_hal_bus::spi::NoDelay>, Output<'_>>, mipidsi::models::ST7796, Output<'_>>,
    in_menu_t:&mut Option<INMENU>,
    in_col_t:&mut Option<INCOLOR>
){
    let style = U8g2TextStyle::new(
        u8g2_fonts::fonts::u8g2_font_helvB18_tr,
        Rgb565::WHITE,
    );
    let in_m = INMENU.lock(|in_m|*in_m.borrow());
    if let Some(in_m)=in_m{
            if in_m==INMENU::COLOR{
                if let None=in_menu_t.as_ref(){
                    embedded_graphics::primitives::Rectangle::new(Point::new(0, 140), Size::new(500, 180))
                    .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                    .draw(display)
                    .unwrap();
                    Text::new("RED        : ", Point::new(50, 180), &style)
                        .draw(display)
                        .unwrap();
                    Text::new("GREEN  : ", Point::new(50, 230), &style)
                        .draw(display)
                        .unwrap();
                    Text::new("BLUE     : ", Point::new(50, 280), &style)
                        .draw(display)
                        .unwrap();
                    *in_menu_t=Some(INMENU::COLOR);
                }else{
                    
                    //IN COLOR MENU DISPLAY
                    let inc = INCOLOR.lock(|inco|*inco.borrow());
                    if *in_col_t != inc{
                        embedded_graphics::primitives::Rectangle::new(Point::new(30, 185), Size::new(150, 20))
                        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                        .draw(display)
                        .unwrap();
                        embedded_graphics::primitives::Rectangle::new(Point::new(30, 235), Size::new(150, 20))
                                .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                                .draw(display)
                                .unwrap();
                        embedded_graphics::primitives::Rectangle::new(Point::new(30, 285), Size::new(150, 20))
                            .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                            .draw(display)
                            .unwrap();
                            // Text::new("RED        : ", Point::new(50, 180), &style)
                            //     .draw(display)
                            //     .unwrap();
                            // Text::new("GREEN  : ", Point::new(50, 230), &style)
                            //     .draw(display)
                            //     .unwrap();
                            // Text::new("BLUE     : ", Point::new(50, 280), &style)
                            //     .draw(display)
                            //     .unwrap();
                        if let Some(in_c)=inc{
                            match in_c {
                                INCOLOR::RED=>{
                                    // embedded_graphics::primitives::Rectangle::new(Point::new(30, 150), Size::new(150, 50))
                                    //     .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                                    //     .draw(display)
                                    //     .unwrap();
                                    // Text::new("RED        : ", Point::new(50, 180), &style)
                                    //     .draw(display)
                                    //     .unwrap();
                                    Text::new("______", Point::new(50, 185), &style)
                                        .draw(display)
                                        .unwrap();
                                    // Text::new("RED", Point::new(50, 180), &style)
                                    //     .draw(display)
                                    //     .unwrap();
                                    },  
                                INCOLOR::GREEN=>{
                                    // embedded_graphics::primitives::Rectangle::new(Point::new(30, 200), Size::new(150, 50))
                                    //     .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                                    //     .draw(display)
                                    //     .unwrap();
                                    // Text::new("GREEN  : ", Point::new(50, 230), &style)
                                    //     .draw(display)
                                    //     .unwrap();
                                    Text::new("______", Point::new(50, 235), &style)
                                        .draw(display)
                                        .unwrap();

                                },
                                INCOLOR::BLUE=>{
                                    // embedded_graphics::primitives::Rectangle::new(Point::new(30, 250), Size::new(150, 50))
                                    //     .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                                    //     .draw(display)
                                    //     .unwrap();
                                    // Text::new("BLUE     : ", Point::new(50, 280), &style)
                                    //     .draw(display)
                                    //     .unwrap();
                                    Text::new("______", Point::new(50, 285), &style)
                                        .draw(display)
                                        .unwrap();

                                }

                            }
                        }
                        *in_col_t=inc;
                    }
                    
                    
                }
            }
        }else{
            Text::new("Click to adjust the color", Point::new(50, 180), &style)
                .draw(display)
                .unwrap();
            *in_menu_t=in_m;
        }
    // INMENU.lock(|i_m|{
    //     let mut m: core::cell::RefMut<'_, Option<INMENU>> = i_m.borrow_mut();
        
    //     // *m=None;
    // });
    
}