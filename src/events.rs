use std::error;
use std::fmt;

use rmpv::Value;
use skulpin::skia_safe::Color4f;

use crate::editor::{Colors, Style, CursorMode, CursorShape};

#[derive(Debug, Clone)]
pub enum EventParseError {
    InvalidArray(Value),
    InvalidMap(Value),
    InvalidString(Value),
    InvalidU64(Value),
    InvalidI64(Value),
    InvalidEventFormat
}
type Result<T> = std::result::Result<T, EventParseError>;

impl fmt::Display for EventParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EventParseError::InvalidArray(value) => write!(f, "invalid array format {}", value),
            EventParseError::InvalidMap(value) => write!(f, "invalid map format {}", value),
            EventParseError::InvalidString(value) => write!(f, "invalid string format {}", value),
            EventParseError::InvalidU64(value) => write!(f, "invalid u64 format {}", value),
            EventParseError::InvalidI64(value) => write!(f, "invalid i64 format {}", value),
            EventParseError::InvalidEventFormat => write!(f, "invalid event format")
        }
    }
}

impl error::Error for EventParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub struct GridLineCell {
    pub text: String,
    pub highlight_id: Option<u64>,
    pub repeat: Option<u64>
}

#[derive(Debug)]
pub enum RedrawEvent {
    SetTitle { title: String },
    ModeInfoSet { cursor_modes: Vec<CursorMode> },
    ModeChange { mode_index: u64 },
    BusyStart,
    BusyStop,
    Flush,
    Resize { grid: u64, width: u64, height: u64 },
    DefaultColorsSet { colors: Colors },
    HighlightAttributesDefine { id: u64, style: Style },
    GridLine { grid: u64, row: u64, column_start: u64, cells: Vec<GridLineCell> },
    Clear { grid: u64 },
    CursorGoto { grid: u64, row: u64, column: u64 },
    Scroll { grid: u64, top: u64, bottom: u64, left: u64, right: u64, rows: i64, columns: i64 }
}

fn unpack_color(packed_color: u64) -> Color4f {
    let packed_color = packed_color as u32;
    let r = ((packed_color & 0xff0000) >> 16) as f32;
    let g = ((packed_color & 0xff00) >> 8) as f32;
    let b = (packed_color & 0xff) as f32;
    Color4f {
        r: r / 255.0,
        g: g / 255.0,
        b: b / 255.0,
        a: 1.0
    }
}

fn parse_array(array_value: &Value) -> Result<Vec<Value>> {
    if let Value::Array(content) = array_value.clone() {
        Ok(content.to_vec())
    } else {
        Err(EventParseError::InvalidArray(array_value.clone()))
    }
}

fn parse_map(map_value: &Value) -> Result<Vec<(Value, Value)>> {
    if let Value::Map(content) = map_value.clone() {
        Ok(content)
    } else {
        Err(EventParseError::InvalidMap(map_value.clone()))
    }
}

fn parse_string(string_value: &Value) -> Result<String> {
    if let Value::String(content) = string_value.clone() {
        Ok(content.into_str().ok_or(EventParseError::InvalidString(string_value.clone()))?)
    } else {
        Err(EventParseError::InvalidString(string_value.clone()))
    }
}

fn parse_u64(u64_value: &Value) -> Result<u64> {
    if let Value::Integer(content) = u64_value.clone() {
        Ok(content.as_u64().ok_or(EventParseError::InvalidU64(u64_value.clone()))?)
    } else {
        Err(EventParseError::InvalidU64(u64_value.clone()))
    }
}

fn parse_i64(i64_value: &Value) -> Result<i64> {
    if let Value::Integer(content) = i64_value.clone() {
        Ok(content.as_i64().ok_or(EventParseError::InvalidI64(i64_value.clone()))?)
    } else {
        Err(EventParseError::InvalidI64(i64_value.clone()))
    }
}

fn parse_set_title(set_title_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [title] = set_title_arguments.as_slice() {
        Ok(RedrawEvent::SetTitle {
            title: parse_string(&title)?
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_mode_info_set(mode_info_set_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [_cursor_style_enabled, mode_info] = mode_info_set_arguments.as_slice() {
        let mode_info_values = parse_array(&mode_info)?;
        let mut cursor_modes = Vec::new();
        for mode_info_value in mode_info_values {
            let info_map = parse_map(&mode_info_value)?;
            let mut mode_info = CursorMode::new();
            for (name, value) in info_map {
                let name = parse_string(&name)?;
                match name.as_ref() {
                    "cursor_shape" => {
                        mode_info.shape = CursorShape::from_type_name(&parse_string(&value)?);
                    },
                    "attr_id" => {
                        mode_info.style_id = Some(parse_u64(&value)?);
                    },
                    _ => {}
                }
            }
            cursor_modes.push(mode_info);
        }
        Ok(RedrawEvent::ModeInfoSet {
            cursor_modes
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_mode_change(mode_change_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [_mode, mode_index] = mode_change_arguments.as_slice() {
        Ok(RedrawEvent::ModeChange {
            mode_index: parse_u64(&mode_index)?
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_grid_resize(grid_resize_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [grid_id, width, height] = grid_resize_arguments.as_slice() {
        Ok(RedrawEvent::Resize { 
            grid: parse_u64(&grid_id)?, width: parse_u64(&width)?, height: parse_u64(&height)?
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_default_colors(default_colors_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [
        foreground, background, special, _term_foreground, _term_background
    ] = default_colors_arguments.as_slice() {
        Ok(RedrawEvent::DefaultColorsSet {
            colors: Colors {
                foreground: Some(unpack_color(parse_u64(&foreground)?)),
                background: Some(unpack_color(parse_u64(&background)?)),
                special: Some(unpack_color(parse_u64(special)?)),
            }
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_hl_attr_define(hl_attr_define_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [
        id, Value::Map(attributes), _terminal_attributes, _info
    ] = hl_attr_define_arguments.as_slice() {
        let mut style = Style::new(Colors::new(None, None, None));
        for attribute in attributes {
            if let (Value::String(name), value) = attribute {
                match (name.as_str().unwrap(), value) {
                    ("foreground", Value::Integer(packed_color)) => style.colors.foreground = Some(unpack_color(packed_color.as_u64().unwrap())),
                    ("background", Value::Integer(packed_color)) => style.colors.background = Some(unpack_color(packed_color.as_u64().unwrap())),
                    ("special", Value::Integer(packed_color)) => style.colors.special = Some(unpack_color(packed_color.as_u64().unwrap())),
                    ("reverse", Value::Boolean(reverse)) => style.reverse = *reverse,
                    ("italic", Value::Boolean(italic)) => style.italic = *italic,
                    ("bold", Value::Boolean(bold)) => style.bold = *bold,
                    ("strikethrough", Value::Boolean(strikethrough)) => style.strikethrough = *strikethrough,
                    ("underline", Value::Boolean(underline)) => style.underline = *underline,
                    ("undercurl", Value::Boolean(undercurl)) => style.undercurl = *undercurl,
                    ("blend", Value::Integer(blend)) => style.blend = blend.as_u64().unwrap() as u8,
                    _ => println!("Ignored style attribute: {}", name)
                }
            } else {
                println!("Invalid attribute format");
            }
        }
        Ok(RedrawEvent::HighlightAttributesDefine { id: parse_u64(&id)?, style })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_grid_line_cell(grid_line_cell: Value) -> Result<GridLineCell> {
    let cell_contents = parse_array(&grid_line_cell)?;
    let text_value = cell_contents.get(0).ok_or(EventParseError::InvalidEventFormat)?;
    Ok(GridLineCell {
        text: parse_string(&text_value)?,
        highlight_id: cell_contents.get(1).map(|highlight_id| parse_u64(highlight_id)).transpose()?,
        repeat: cell_contents.get(2).map(|repeat| parse_u64(repeat)).transpose()?
    })
}

fn parse_grid_line(grid_line_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [grid_id, row, column_start, cells] = grid_line_arguments.as_slice() {
        Ok(RedrawEvent::GridLine {
            grid: parse_u64(&grid_id)?, 
            row: parse_u64(&row)?, column_start: parse_u64(&column_start)?,
            cells: parse_array(&cells)?
                .into_iter()
                .map(parse_grid_line_cell)
                .collect::<Result<Vec<GridLineCell>>>()?
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_clear(clear_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [grid_id] = clear_arguments.as_slice() {
        Ok(RedrawEvent::Clear { grid: parse_u64(&grid_id)? })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_cursor_goto(cursor_goto_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [grid_id, column, row] = cursor_goto_arguments.as_slice() {
        Ok(RedrawEvent::CursorGoto { 
            grid: parse_u64(&grid_id)?, row: parse_u64(&row)?, column: parse_u64(&column)?
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

fn parse_grid_scroll(grid_scroll_arguments: Vec<Value>) -> Result<RedrawEvent> {
    if let [grid_id, top, bottom, left, right, rows, columns] = grid_scroll_arguments.as_slice() {
        Ok(RedrawEvent::Scroll {
            grid: parse_u64(&grid_id)?, 
            top: parse_u64(&top)?, bottom: parse_u64(&bottom)?,
            left: parse_u64(&left)?, right: parse_u64(&right)?,
            rows: parse_i64(&rows)?, columns: parse_i64(&columns)?
        })
    } else {
        Err(EventParseError::InvalidEventFormat)
    }
}

pub fn parse_redraw_event(event_value: Value) -> Result<Vec<RedrawEvent>> {
    let event_contents = parse_array(&event_value)?.to_vec();
    let name_value = event_contents.get(0).ok_or(EventParseError::InvalidEventFormat)?;
    let event_name = parse_string(&name_value)?;
    let events = event_contents;
    let mut parsed_events = Vec::new();

    for event in &events[1..] {
        let event_parameters = parse_array(&event)?;
        let possible_parsed_event = match event_name.clone().as_ref() {
            "set_title" => Some(parse_set_title(event_parameters)?),
            "set_icon" => None, // Ignore set icon for now
            "mode_info_set" => Some(parse_mode_info_set(event_parameters)?),
            "option_set" => None, // Ignore option set for now
            "mode_change" => Some(parse_mode_change(event_parameters)?),
            "busy_start" => Some(RedrawEvent::BusyStart),
            "busy_stop" => Some(RedrawEvent::BusyStop),
            "flush" => Some(RedrawEvent::Flush),
            "grid_resize" => Some(parse_grid_resize(event_parameters)?),
            "default_colors_set" => Some(parse_default_colors(event_parameters)?),
            "hl_attr_define" => Some(parse_hl_attr_define(event_parameters)?),
            "grid_line" => Some(parse_grid_line(event_parameters)?),
            "grid_clear" => Some(parse_clear(event_parameters)?),
            "grid_cursor_goto" => Some(parse_cursor_goto(event_parameters)?),
            "grid_scroll" => Some(parse_grid_scroll(event_parameters)?),
            _ => None
        };

        if let Some(parsed_event) = possible_parsed_event {
            parsed_events.push(parsed_event);
        } else {
            println!("Did not parse {}", event_name);
        }
    }

    Ok(parsed_events)
}

pub fn parse_neovim_event(event_name: String, events: Vec<Value>) -> Result<Vec<RedrawEvent>> {
    let mut resulting_events = Vec::new();
    if event_name == "redraw" {
        for event in events {
            resulting_events.append(&mut parse_redraw_event(event)?);
        }
    } else {
        println!("Unknown global event {}", event_name);
    }
    Ok(resulting_events)
}





