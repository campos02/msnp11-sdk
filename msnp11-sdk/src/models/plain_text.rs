/// Represents a plain text message. Colors are defined in RGB hex(converted to BGR internally).
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PlainText {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: String,
    pub text: String,
}

impl PlainText {
    pub(crate) fn new(payload: String) -> Self {
        let im_format = payload
            .lines()
            .nth(2)
            .unwrap_or_default()
            .replace("X-MMS-IM-Format: ", "");

        let mut im_format = im_format.split(";");
        let formatting = im_format.nth(1).unwrap_or_default();
        let color = im_format.next().unwrap_or_default();

        let bold = formatting.contains("B");
        let italic = formatting.contains("I");
        let underline = formatting.contains("U");
        let strikethrough = formatting.contains("S");
        let mut color = color.replace("CO=", "").trim().to_string();

        while color.len() < 6 {
            color.insert(0, '0');
        }

        // MSN uses BGR... just why
        let color: u32 = if color.len() <= 6 {
            let color = color.drain(..6);
            let color = color.as_str();
            let color = u32::from_str_radix(color, 16).unwrap_or(0);

            let r = (color & 0xFF0000) >> 16;
            let b = (color & 0x0000FF) << 16;
            let g = color & 0x00FF00;
            r | g | b
        } else {
            0
        };

        let color = format!("{color:x}");
        let text = payload
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or_default()
            .replace("\r\n", "\n");

        Self {
            bold,
            italic,
            underline,
            strikethrough,
            color,
            text,
        }
    }

    pub(crate) fn payload(&self) -> String {
        let mut message = String::from("MIME-Version: 1.0\r\n");
        message.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
        message.push_str("X-MMS-IM-Format: FN=Microsoft%20Sans%20Serif; EF=");

        if self.bold {
            message.push('B');
        }

        if self.italic {
            message.push('I');
        }

        if self.underline {
            message.push('U');
        }

        if self.strikethrough {
            message.push('S');
        }

        let mut color = self.color.trim().replace("#", "");
        while color.len() < 6 {
            color.insert(0, '0');
        }

        // MSN uses BGR... just why
        let color: u32 = if self.color.len() <= 6 {
            let color = color.drain(..6);
            let color = color.as_str();
            let color = u32::from_str_radix(color, 16).unwrap_or(0);

            let r = (color & 0xFF0000) >> 16;
            let b = (color & 0x0000FF) << 16;
            let g = color & 0x00FF00;
            b | g | r
        } else {
            0
        };

        let color = format!("{color:x}");
        message.push_str(format!("; CO={color}; CS=1; PF=0\r\n\r\n").as_str());
        message.push_str(
            self.text
                .replace("\n", "\r\n")
                .replace("\r\r", "\r")
                .as_str(),
        );

        message
    }
}
