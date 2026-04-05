use smart_leds_trait::RGB8;

use crate::RGB_DATA;

const LED_N: usize = 17;

pub fn scale8(value: u8, scale: u8) -> u8 {
    ((value as u16 * scale as u16) / 255) as u8
}

pub fn scale_rgb(rgb: (u8, u8, u8), scale: u8) -> RGB8 {
    RGB8::new(
        scale8(rgb.0, scale),
        scale8(rgb.1, scale),
        scale8(rgb.2, scale),
    )
}

pub fn triangle_u8(x: u8) -> u8 {
    if x < 128 {
        x.saturating_mul(2)
    } else {
        (255 - x).saturating_mul(2)
    }
}

pub fn fill_solid(frame: &mut [RGB8; LED_N], rgb: (u8, u8, u8)) {
    for px in frame.iter_mut() {
        *px = RGB8::new(rgb.0, rgb.1, rgb.2);
    }
}

// pub fn fill_wave(frame: &mut [RGB8; LED_N], rgb: (u8, u8, u8), tick: u8) {
//     for i in 0..LED_N {
//         let phase = tick.wrapping_add((i as u8).wrapping_mul(14));
//         let amp = triangle_u8(phase);

//         // 완전 꺼지지 않게 바닥 밝기 조금 줌
//         let amp = 30u8.saturating_add(scale8(amp, 225));
//         frame[i] = scale_rgb(rgb, amp);
//     }
// }

pub fn fill_wave(frame: &mut [RGB8; LED_N], tick: u8, base: (u8, u8, u8)) {
    for i in 0..LED_N {
        let phase = tick.wrapping_add((i as u8).wrapping_mul(5));
        let amp = triangle_u8(phase);
        let amp = 50u8.saturating_add(scale8(amp, 205));

        frame[i] = RGB8::new(
            scale8(base.0, amp),
            scale8(base.1, amp),
            scale8(base.2, amp),
        );
    }
}


pub fn fill_dynamic(frame: &mut [RGB8; LED_N], level: u8) {
    let base = RGB_DATA.lock(|d| *d.borrow());

    let px = RGB8::new(
        scale8(base.0, level),
        scale8(base.1, level),
        scale8(base.2, level),
    );

    for item in frame.iter_mut() {
        *item = px;
    }
}

pub fn update_breath(level: &mut u8, up: &mut bool) {
    if *up {
        *level = level.saturating_add(5);
        if *level >= 240 {
            *level = 240;
            *up = false;
        }
    } else {
        *level = level.saturating_sub(5);
        if *level <= 20 {
            *level = 20;
            *up = true;
        }
    }
}



pub fn wheel(pos: u8) -> RGB8 {
    if pos < 85 {
        RGB8::new(255 - pos * 3, pos * 3, 0)
    } else if pos < 170 {
        let p = pos - 85;
        RGB8::new(0, 255 - p * 3, p * 3)
    } else {
        let p = pos - 170;
        RGB8::new(p * 3, 0, 255 - p * 3)
    }
}

pub fn fill_dynamic_color(frame: &mut [RGB8; LED_N], tick: u8) {
    let color = wheel(tick);

    for px in frame.iter_mut() {
        *px = color;
    }
}