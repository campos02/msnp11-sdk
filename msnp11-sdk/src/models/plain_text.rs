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
            .unwrap_or("")
            .replace("X-MMS-IM-Format: ", "");

        let im_format: Vec<&str> = im_format.split(";").collect();

        let bold = im_format[1].contains("B");
        let italic = im_format[1].contains("I");
        let underline = im_format[1].contains("U");
        let strikethrough = im_format[1].contains("S");
        let mut color = im_format[2].replace("CO=", "").trim().to_string();

        while color.len() < 6 {
            color.insert_str(0, "0");
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
            .unwrap_or("")
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
            message.push_str("B");
        }

        if self.italic {
            message.push_str("I");
        }

        if self.underline {
            message.push_str("U");
        }

        if self.strikethrough {
            message.push_str("S");
        }

        let mut color = self.color.trim().replace("#", "");
        while color.len() < 6 {
            color.insert_str(0, "0");
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
