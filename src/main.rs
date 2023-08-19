use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use nu_plugin::{serve_plugin, EvaluatedCall, JsonSerializer, LabeledError, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, SyntaxShape, Type, Value};

struct Qr;

impl Qr {
    fn new() -> Self {
        Self {}
    }
}

impl Plugin for Qr {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![
            PluginSignature::build("from qr")
            .usage("decode input qr image")
            .category(Category::Strings)
            .input_output_types(vec![
                (Type::Binary, Type::Binary),
                (Type::Binary, Type::String),
            ])
            .switch("ignore-error", "ignore errors if some parts are decodable", Some('i'))
            .plugin_examples(vec![
                PluginExample {
                    description: "convert input string to qr image".into(),
                    example: "open --raw qrcode.png | from qr".into(),
                    result: None,
                },
            ]),
            PluginSignature::build("to qr")
            .usage("convert input to png image of qr code")
            .category(Category::Strings)
            .input_output_types(vec![
                (Type::Binary, Type::Binary),
                (Type::String, Type::Binary),
            ])
            .named("shape", SyntaxShape::String, "allowed: Square(Default), Circle, RoundedSquare, Vertical, Horizontal, Diamond", Some('s'))
            .named("width", SyntaxShape::Int, "Target width", Some('w'))
            .named("height", SyntaxShape::Int, "Target height", Some('v'))
            .named("background", SyntaxShape::List(Box::new(SyntaxShape::Int)), "background coler", Some('b'))
            .plugin_examples(vec![
                PluginExample {
                    description: "convert string to qr code, default width is 600".into(),
                    example: "\"hello!\" | to qr | save qr.png".into(),
                    result: None,
                },
                PluginExample {
                    description: "convert string to qr code with given shape and width".into(),
                    example: "\"hello!\" | to qr --shape circle --width 300 | save qr.png".into(),
                    result: None,
                }
            ]),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let input_span = Some(input.span().unwrap_or(call.head));
        match name {
            "from qr" => {
                let ignore_error = call.has_flag("ignore-error");
                let bytes = input.as_binary()?;
                let format = image::guess_format(bytes)
                    .map(|x| (x.extensions_str(), x.to_mime_type()))
                    .unwrap_or((&[], "unknown"));
                match image::load_from_memory(bytes) {
                    Ok(image) => {
                        let image = image.into_luma8();
                        let mut decoder = quircs::Quirc::default();
                        let mut v = Vec::new();
                        for s in decoder.identify(
                            image.width() as usize,
                            image.height() as usize,
                            &image,
                        ) {
                            match s {
                                Ok(data) => match data.decode() {
                                    Ok(data) => v.push(data.payload),
                                    Err(e) => {
                                        if !ignore_error {
                                            return Err(LabeledError {
                                                label: "input contains incorrect data".into(),
                                                msg: format!(
                                                    "identified data can not be decoded: {}",
                                                    e
                                                ),
                                                span: input_span,
                                            });
                                        } else {
                                            eprintln!("Ignore error while decoding: {}", e);
                                        }
                                    }
                                },
                                Err(e) => {
                                    if !ignore_error {
                                        return Err(LabeledError {
                                            label: "input contains incorrect data".into(),
                                            msg: format!(
                                                "part of data can not be identified: {}",
                                                e
                                            ),
                                            span: input_span,
                                        });
                                    } else {
                                        eprintln!("Ignore error while decoding: {}", e);
                                    }
                                }
                            }
                        }
                        let mut string_buf = Vec::new();
                        for data in v.iter() {
                            if let Ok(s) = String::from_utf8(data.clone()) {
                                string_buf.push(s);
                            } else {
                                break;
                            }
                        }
                        Ok(if string_buf.len() == v.len() {
                            Value::String {
                                val: string_buf.join("\n"),
                                span: call.head,
                            }
                        } else {
                            Value::Binary {
                                val: v.into_iter().flatten().collect::<Vec<u8>>(),
                                span: call.head,
                            }
                        })
                    }
                    Err(e) => Err(LabeledError {
                        label: format!("Unable to open image: {}", e),
                        msg: format!("Input is guessed as {}", format_image(format.1, format.0)),
                        span: input_span,
                    }),
                }
            }
            "to qr" => {
                let input = input.as_binary()?;
                let shape_name: Option<String> = call.get_flag("shape")?;
                let shape = match shape_name.map(|x| x.to_uppercase()).as_deref() {
                    Some("SQUARE") => Shape::Square,
                    Some("CIRCLE") => Shape::Circle,
                    Some("ROUNDEDSQUARE") => Shape::RoundedSquare,
                    Some("VERTICAL") => Shape::Vertical,
                    Some("HORIZONTAL") => Shape::Horizontal,
                    Some("DIAMOND") => Shape::Diamond,
                    None => Shape::Square,
                    _ => {
                        return Err(LabeledError {
                            label: "Unknown shape parameter".into(),
                            msg: "should be one of Square, Circle, RoundedSquare, Vertical, Horizontal, Diamond".into(),
                            span: Some(call.head),
                        })
                    }
                };
                /* 
                let (r,g,b,a) = match call.get_flag_value("background") {
                    Some(Value::List { vals, .. }) => {
                        match vals.len() {
                            3 => {
                                let v : Vec<usize> = vals.into_iter().map(|x| x.as_int()).collect();
                                (v[0], v[1], v[2], 0)
                            },
                            4 => {
                                let v : Vec<usize> = vals.into_iter().map(|x| x.as_int()).collect();
                                (v[0], v[1], v[2], v[3])
                            },
                            _ => {
                                return Err(LabeledError { label: "incorrect background".into(), msg: "Sholud be a list of [r g b] or [r g b a]".into(), span: Some(call.head) })
                            }
                        }
                    },
                    Some(_) => {
                        return Err(LabeledError { label: "incorrect background".into(), msg: "Sholud be a list of [r g b] or [r g b a]".into(), span: Some(call.head) })
                    },
                    None => {
                        (255.255,255,0)
                    }
                };
                */
                match fast_qr::QRBuilder::new(input).build() {
                    Ok(image) => {
                        let mut builder = ImageBuilder::default();
                        builder.shape(shape);
                        //builder.background_color([r,g,b,a]);
                        match (
                            call.get_flag::<usize>("width")?,
                            call.get_flag::<usize>("height")?,
                        ) {
                            (Some(w), Some(h))
                                if w < u32::MAX as usize && h < u32::MAX as usize =>
                            {
                                builder.fit_width(w as u32).fit_width(h as u32)
                            }
                            (Some(w), None) if w < u32::MAX as usize => builder.fit_width(w as u32),
                            (None, Some(h)) if h < u32::MAX as usize => {
                                builder.fit_height(h as u32)
                            }
                            (None, None) => builder.fit_width(600),
                            _ => {
                                return Err(LabeledError {
                                    label: "Invalid width/height: too large".into(),
                                    msg: format!(
                                        "width/height should be smaller than {}",
                                        u32::MAX
                                    ),
                                    span: Some(call.head),
                                })
                            }
                        };
                        match builder.to_pixmap(&image).encode_png() {
                            Ok(buf) => Ok(Value::Binary {
                                val: buf,
                                span: call.head,
                            }),
                            Err(e) => Err(LabeledError {
                                label: "failed to generate png".into(),
                                msg: e.to_string(),
                                span: Some(call.head),
                            }),
                        }
                    }
                    Err(e) => Err(LabeledError {
                        label: "failed to generate qr code".into(),
                        msg: e.to_string(),
                        span: input_span,
                    }),
                }
            }
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "Plugin command does not exist".into(),
                span: Some(call.head),
            }),
        }
    }
}

fn format_image(format: &str, extension: &[&str]) -> String {
    match extension.len() {
        0 => {
            format!("{} (unknown extension)", format)
        }
        _ => {
            format!("{} ({})", format, extension.join(", "))
        }
    }
}

fn main() {
    serve_plugin(&mut Qr::new(), JsonSerializer)
}
