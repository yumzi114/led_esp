use embedded_graphics::{pixelcolor::Rgb565, prelude::{Point, Primitive, RgbColor as _, Size}, text::Text};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{gpio::Output, spi::master::Spi};
use u8g2_fonts::U8g2TextStyle;
use core::fmt::Write;
use crate::{BRIGHTNESS, INCOLOR, INMENU, RGB_DATA};
use embedded_graphics::Drawable;




pub fn main_body (
    display: &mut mipidsi::Display<mipidsi::interface::SpiInterface<'_, ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, embedded_hal_bus::spi::NoDelay>, Output<'_>>, mipidsi::models::ST7796, Output<'_>>
){
    let style = U8g2TextStyle::new(
        u8g2_fonts::fonts::u8g2_font_helvB18_tr,
        Rgb565::WHITE,
    );
    Text::new("BRIGHTNESS : ", Point::new(50, 180), &style)
        .draw(display)
        .unwrap();
    let text = BRIGHTNESS.lock(|data|*data.borrow());
    let mut buf = heapless::String::<8>::new();
    // let text = *b.borrow();
    write!(buf, "{}", text).unwrap();
    // write!(buf, "{}", val).unwrap();
    
    Text::new(&buf, Point::new(250, 180), &style)
        .draw(display)
        .unwrap();
    let mut buf = heapless::String::<18>::new();
    let text = RGB_DATA.lock(|data|*data.borrow());
    write!(buf, "{:?}", text).unwrap();
    Text::new("COLOR : ", Point::new(50, 230), &style)
        .draw(display)
        .unwrap();
    Text::new("RGB - ", Point::new(200, 230), &style)
        .draw(display)
        .unwrap();
    Text::new(&buf, Point::new(280, 230), &style)
        .draw(display)
        .unwrap();
    
    Text::new("MODE : ", Point::new(50, 280), &style)
        .draw(display)
        .unwrap();
    
}





pub fn bri_body (
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

pub fn color_body (
    display: &mut mipidsi::Display<mipidsi::interface::SpiInterface<'_, ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, embedded_hal_bus::spi::NoDelay>, Output<'_>>, mipidsi::models::ST7796, Output<'_>>,
    in_menu_t:&mut Option<INMENU>,
    in_col_t:&mut Option<INCOLOR>,
    flag_rgb_data:&mut (u8,u8,u8)
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
                    let rgb = RGB_DATA.lock(|data|*data.borrow());
                    embedded_graphics::primitives::Rectangle::new(Point::new(200, 150), Size::new(100, 40))
                        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                        .draw(display)
                        .unwrap();
                    let mut buf = heapless::String::<8>::new();
                    let text = rgb.0;
                    write!(buf, "{}", text).unwrap();
                    // write!(buf, "{}", val).unwrap();
                    Text::new(&buf, Point::new(250, 180), &style)
                        .draw(display)
                        .unwrap();
                    embedded_graphics::primitives::Rectangle::new(Point::new(200, 200), Size::new(100, 40))
                        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                        .draw(display)
                        .unwrap();
                    let mut buf = heapless::String::<8>::new();
                    let text = rgb.1;
                    write!(buf, "{}", text).unwrap();
                    // write!(buf, "{}", val).unwrap();
                    Text::new(&buf, Point::new(250, 230), &style)
                        .draw(display)
                        .unwrap();
                    embedded_graphics::primitives::Rectangle::new(Point::new(200, 250), Size::new(100, 40))
                        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                        .draw(display)
                        .unwrap();
                    let mut buf = heapless::String::<8>::new();
                    let text = rgb.2;
                    write!(buf, "{}", text).unwrap();
                    // write!(buf, "{}", val).unwrap();
                    Text::new(&buf, Point::new(250, 280), &style)
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
                
                        if let Some(in_c)=inc{
                            match in_c {
                                INCOLOR::RED=>{
                  
                                    Text::new("______", Point::new(50, 185), &style)
                                        .draw(display)
                                        .unwrap();
                       
                                    },  
                                INCOLOR::GREEN=>{
              
                                    Text::new("______", Point::new(50, 235), &style)
                                        .draw(display)
                                        .unwrap();

                                },
                                INCOLOR::BLUE=>{
                       
                                    Text::new("______", Point::new(50, 285), &style)
                                        .draw(display)
                                        .unwrap();

                                }

                            }
                        }
                        *in_col_t=inc;
                    }

                    if let Some(inc)=inc{
                        let rgb = RGB_DATA.lock(|data|*data.borrow());
                        if *flag_rgb_data !=rgb{
                            
                            match inc {
                                INCOLOR::RED=>{
                                    embedded_graphics::primitives::Rectangle::new(Point::new(200, 150), Size::new(100, 40))
                                        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                                        .draw(display)
                                        .unwrap();
                                    let mut buf = heapless::String::<8>::new();
                                    let text = rgb.0;
                                    write!(buf, "{}", text).unwrap();
                                    // write!(buf, "{}", val).unwrap();
                                    Text::new(&buf, Point::new(250, 180), &style)
                                        .draw(display)
                                        .unwrap();
                                    // Text::new("______", Point::new(15, 180), &style)
                                    // .draw(display)
                                    // .unwrap();
                                    *flag_rgb_data=rgb;
                                },
                                INCOLOR::GREEN=>{
                                    embedded_graphics::primitives::Rectangle::new(Point::new(200, 200), Size::new(100, 40))
                                        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                                        .draw(display)
                                        .unwrap();
                                    let mut buf = heapless::String::<8>::new();
                                    let text = rgb.1;
                                    write!(buf, "{}", text).unwrap();
                                    // write!(buf, "{}", val).unwrap();
                                    Text::new(&buf, Point::new(250, 230), &style)
                                        .draw(display)
                                        .unwrap();
                                    // Text::new("______", Point::new(15, 180), &style)
                                    // .draw(display)
                                    // .unwrap();
                                    *flag_rgb_data=rgb;
                                },
                                INCOLOR::BLUE=>{
                                    embedded_graphics::primitives::Rectangle::new(Point::new(200, 250), Size::new(100, 40))
                                        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                                        .draw(display)
                                        .unwrap();
                                    let mut buf = heapless::String::<8>::new();
                                    let text = rgb.2;
                                    write!(buf, "{}", text).unwrap();
                                    // write!(buf, "{}", val).unwrap();
                                    Text::new(&buf, Point::new(250, 280), &style)
                                        .draw(display)
                                        .unwrap();
                                    // Text::new("______", Point::new(15, 180), &style)
                                    // .draw(display)
                                    // .unwrap();
                                    *flag_rgb_data=rgb;
                                },
                            }
                        }
                        // flag_rgb_data=rgb;
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



pub fn mode_body (
    display: &mut mipidsi::Display<mipidsi::interface::SpiInterface<'_, ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, embedded_hal_bus::spi::NoDelay>, Output<'_>>, mipidsi::models::ST7796, Output<'_>>,
    in_menu_t:&mut Option<INMENU>,
){
    let style = U8g2TextStyle::new(
        u8g2_fonts::fonts::u8g2_font_helvB18_tr,
        Rgb565::WHITE,
    );
    INMENU.lock(|i_m|{
        let mut m = i_m.borrow_mut();
        if let Some(in_m)=*m{
            if in_m==INMENU::MODE{
                if let None=in_menu_t.as_ref(){
                    embedded_graphics::primitives::Rectangle::new(Point::new(0, 140), Size::new(500, 180))
                    .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(Rgb565::RED))
                    .draw(display)
                    .unwrap();
                    Text::new("MODE : ", Point::new(50, 180), &style)
                        .draw(display)
                        .unwrap();
                    *in_menu_t=Some(INMENU::MODE);
                }
                
            }
            
            
        }else{
            Text::new("Click to adjust the Mode", Point::new(50, 180), &style)
                .draw(display)
                .unwrap();
            *in_menu_t=*m;
        }
        // *m=None;
    });
}
