//! 二维码解码（纯 Rust，rqrr）。
//!
//! Android 侧只负责把 Bitmap 转成灰度字节数组，识别全部在这里完成，
//! 这样不引入 Java 第三方库，也能保持 release 体积可控。

use rqrr::PreparedImage;

/// 从灰度像素中解出第一个可识别的二维码文本。
///
/// `data` 按行优先排列，长度必须 >= `width * height`；不满足直接返回 None。
pub fn decode_gray(width: usize, height: usize, data: &[u8]) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }
    let need = width.checked_mul(height)?;
    if data.len() < need {
        return None;
    }
    let mut img = PreparedImage::prepare_from_greyscale(width, height, |x, y| data[y * width + x]);
    for grid in img.detect_grids() {
        if let Ok((_meta, content)) = grid.decode() {
            let text = content.trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use qrcode::QrCode;

    fn render(text: &str) -> (usize, usize, Vec<u8>) {
        let code = QrCode::new(text.as_bytes()).expect("encode qr");
        let colors = code.to_colors();
        let modules = code.width();
        let scale = 6usize;
        let quiet = 4usize;
        let side = (modules + quiet * 2) * scale;
        let mut buf = vec![255u8; side * side];
        for y in 0..modules {
            for x in 0..modules {
                if colors[y * modules + x] == qrcode::Color::Dark {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = (x + quiet) * scale + dx;
                            let py = (y + quiet) * scale + dy;
                            buf[py * side + px] = 0;
                        }
                    }
                }
            }
        }
        (side, side, buf)
    }

    #[test]
    fn decodes_ascii() {
        let (w, h, gray) = render("https://win.9xueqi.com/sign?activityId=1&userId=2&sp=abc");
        assert_eq!(
            decode_gray(w, h, &gray).as_deref(),
            Some("https://win.9xueqi.com/sign?activityId=1&userId=2&sp=abc")
        );
    }

    #[test]
    fn decodes_utf8() {
        let (w, h, gray) = render("澜彩哨兵-签到");
        assert_eq!(decode_gray(w, h, &gray).as_deref(), Some("澜彩哨兵-签到"));
    }

    #[test]
    fn rejects_blank_image() {
        let side = 200usize;
        let white = vec![255u8; side * side];
        assert_eq!(decode_gray(side, side, &white), None);
    }

    #[test]
    fn rejects_bad_dimensions() {
        assert_eq!(decode_gray(0, 10, &[0u8; 10]), None);
        assert_eq!(decode_gray(10, 10, &[0u8; 5]), None);
    }
}
