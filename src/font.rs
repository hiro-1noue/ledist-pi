use crate::RgbFrame;
use anyhow::{Result, bail};
use encoding_rs::EUC_JP;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct BdfFont {
    glyphs: BTreeMap<u32, Glyph>,
    jis_x0208: bool,
}
#[derive(Clone, Debug)]
struct Glyph {
    width: usize,
    height: usize,
    rows: Vec<u32>,
}
impl BdfFont {
    pub fn parse_bdf(source: &str) -> Result<Self> {
        let mut glyphs = BTreeMap::new();
        let mut jis_x0208 = false;
        let mut encoding = None;
        let mut width = 0usize;
        let mut height = 0usize;
        let mut rows = Vec::new();
        let mut bitmap = false;
        for line in source.lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            match words.as_slice() {
                ["CHARSET_REGISTRY", registry] => {
                    jis_x0208 = registry.trim_matches('"').starts_with("JISX0208");
                }
                ["STARTCHAR", ..] => {
                    encoding = None;
                    width = 0;
                    height = 0;
                    rows.clear();
                    bitmap = false;
                }
                ["ENCODING", code] => encoding = code.parse::<u32>().ok(),
                ["BBX", w, h, ..] => {
                    width = w.parse()?;
                    height = h.parse()?;
                }
                ["BITMAP"] => bitmap = true,
                ["ENDCHAR"] => {
                    if let Some(code) = encoding {
                        if rows.len() != height {
                            bail!("glyph {code} has invalid bitmap height");
                        }
                        glyphs.insert(
                            code,
                            Glyph {
                                width,
                                height,
                                rows: rows.clone(),
                            },
                        );
                    }
                    bitmap = false;
                }
                _ if bitmap => rows.push(u32::from_str_radix(line.trim(), 16)?),
                _ => {}
            }
        }
        Ok(Self { glyphs, jis_x0208 })
    }
    fn glyph(&self, ch: char) -> Option<&Glyph> {
        let code = if self.jis_x0208 {
            let text = ch.to_string();
            let encoded = EUC_JP.encode(&text).0;
            let bytes = encoded.as_ref();
            if bytes.len() == 2 && bytes[0] >= 0xa1 && bytes[1] >= 0xa1 {
                u32::from(bytes[0] - 0x80) << 8 | u32::from(bytes[1] - 0x80)
            } else {
                ch as u32
            }
        } else {
            ch as u32
        };
        self.glyphs.get(&code)
    }
    pub fn measure(&self, text: &str) -> usize {
        text.chars()
            .filter_map(|ch| self.glyph(ch))
            .map(|g| g.width)
            .sum()
    }
    pub fn merge_fallback(&mut self, fallback: Self) {
        self.glyphs.extend(fallback.glyphs);
    }
    pub fn measure_checked(&self, text: &str) -> Result<usize> {
        let mut total = 0;
        for ch in text.chars() {
            total += self
                .glyph(ch)
                .ok_or_else(|| anyhow::anyhow!("font has no glyph for {ch}"))?
                .width;
        }
        Ok(total)
    }
    pub fn height(&self) -> usize {
        self.glyphs
            .values()
            .map(|glyph| glyph.height)
            .max()
            .unwrap_or(0)
    }
    pub fn draw(
        &self,
        text: &str,
        frame: &mut RgbFrame,
        x: isize,
        y: isize,
        color: [u8; 3],
    ) -> Result<()> {
        self.measure_checked(text)?;
        let mut pen = x;
        for character in text.chars() {
            let glyph = self.glyph(character).expect("checked above");
            for (gy, row) in glyph.rows.iter().enumerate() {
                for gx in 0..glyph.width {
                    if row & (1 << (glyph.width.saturating_sub(1) - gx)) == 0 {
                        continue;
                    }
                    let dx = pen + gx as isize;
                    let dy = y + gy as isize;
                    if dx >= 0
                        && dy >= 0
                        && (dx as usize) < frame.width()
                        && (dy as usize) < frame.height()
                    {
                        frame.blit_rgb(dx, dy, 1, 1, &color)?;
                    }
                }
            }
            pen += glyph.width as isize;
        }
        Ok(())
    }
}
