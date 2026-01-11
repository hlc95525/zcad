//! CAD输入解析器
//!
//! 支持多种输入格式：
//! - 绝对坐标: `100,50`
//! - 相对坐标: `@100,50`
//! - 极坐标: `@100<45` (相对) 或 `100<45` (绝对)
//! - 长度: `100`
//! - 角度: `<45`
//! - 长度+角度: `100<45`
//! - 尺寸: `100,50` (用于矩形宽高)

use crate::math::Point2;

/// 解析后的输入值
#[derive(Debug, Clone, PartialEq)]
pub enum InputValue {
    /// 点坐标
    Point(Point2),
    /// 长度值
    Length(f64),
    /// 角度值（弧度）
    Angle(f64),
    /// 长度和角度（弧度）
    LengthAngle { length: f64, angle: f64 },
    /// 尺寸（宽高）
    Dimensions { width: f64, height: f64 },
}

/// 解析错误
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// 无效格式
    InvalidFormat(String),
    /// 缺少必需的值
    MissingValue(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ParseError::MissingValue(msg) => write!(f, "Missing value: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

/// 输入解析器
pub struct InputParser;

impl InputParser {
    /// 解析输入字符串
    ///
    /// # 参数
    /// - `input`: 输入字符串
    /// - `reference_point`: 参考点（用于相对坐标和极坐标）
    ///
    /// # 返回
    /// 解析后的输入值或错误
    pub fn parse(input: &str, reference_point: Option<Point2>) -> Result<InputValue, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidFormat("Empty input".to_string()));
        }

        // 尝试解析为长度+角度格式 (如 "100<45" 或 "@100<45")
        if let Some(angle_pos) = input.rfind('<') {
            let (prefix, angle_str) = input.split_at(angle_pos);
            let angle_str = &angle_str[1..]; // 去掉 '<'

            // 解析角度
            let angle_deg = angle_str
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidFormat(format!("Invalid angle: {}", angle_str)))?;
            let angle_rad = angle_deg.to_radians();

            // 检查是否有长度部分
            if prefix.is_empty() {
                // 只有角度: "<45"
                return Ok(InputValue::Angle(angle_rad));
            }

            // 检查是否是相对坐标
            let (is_relative, length_str) = if prefix.starts_with('@') {
                (true, &prefix[1..])
            } else {
                (false, prefix)
            };

            if length_str.is_empty() {
                return Ok(InputValue::Angle(angle_rad));
            }

            // 解析长度
            let length = length_str
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidFormat(format!("Invalid length: {}", length_str)))?;

            if is_relative {
                // 相对极坐标: "@100<45"
                if let Some(ref_point) = reference_point {
                    let point = Self::polar_to_point(ref_point, length, angle_rad);
                    return Ok(InputValue::Point(point));
                } else {
                    return Err(ParseError::MissingValue(
                        "Reference point required for relative polar coordinate".to_string(),
                    ));
                }
            } else {
                // 长度+角度: "100<45"
                return Ok(InputValue::LengthAngle {
                    length,
                    angle: angle_rad,
                });
            }
        }

        // 尝试解析为坐标格式 (如 "100,50" 或 "@100,50")
        if let Some(comma_pos) = input.find(',') {
            let (x_str, y_str) = input.split_at(comma_pos);
            let y_str = &y_str[1..]; // 去掉 ','

            // 检查是否是相对坐标
            let (is_relative, x_str_clean) = if x_str.starts_with('@') {
                (true, &x_str[1..])
            } else {
                (false, x_str)
            };

            // 解析X坐标
            let x = x_str_clean
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidFormat(format!("Invalid X coordinate: {}", x_str_clean)))?;

            // 解析Y坐标
            let y = y_str
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidFormat(format!("Invalid Y coordinate: {}", y_str)))?;

            if is_relative {
                // 相对坐标: "@100,50"
                if let Some(ref_point) = reference_point {
                    let point = Point2::new(ref_point.x + x, ref_point.y + y);
                    return Ok(InputValue::Point(point));
                } else {
                    return Err(ParseError::MissingValue(
                        "Reference point required for relative coordinate".to_string(),
                    ));
                }
            } else {
                // 绝对坐标: "100,50"
                // 检查是否可能是尺寸（当有参考点时，可能是尺寸输入）
                if reference_point.is_some() {
                    // 可能是尺寸输入，但这里我们返回点，让调用者根据上下文判断
                    return Ok(InputValue::Point(Point2::new(x, y)));
                } else {
                    return Ok(InputValue::Point(Point2::new(x, y)));
                }
            }
        }

        // 尝试解析为纯数字（长度或半径）
        if let Ok(value) = input.parse::<f64>() {
            return Ok(InputValue::Length(value));
        }

        Err(ParseError::InvalidFormat(format!("Cannot parse input: {}", input)))
    }

    /// 解析为点坐标（强制返回点）
    ///
    /// 如果输入是长度+角度，会基于参考点计算点坐标
    pub fn parse_point(
        input: &str,
        reference_point: Option<Point2>,
    ) -> Result<Point2, ParseError> {
        match Self::parse(input, reference_point)? {
            InputValue::Point(p) => Ok(p),
            InputValue::LengthAngle { length, angle } => {
                if let Some(ref_point) = reference_point {
                    Ok(Self::polar_to_point(ref_point, length, angle))
                } else {
                    Err(ParseError::MissingValue(
                        "Reference point required for length+angle input".to_string(),
                    ))
                }
            }
            InputValue::Length(len) => {
                // 如果只有长度，需要参考点和当前方向
                // 这里我们假设是水平方向（0度）
                if let Some(ref_point) = reference_point {
                    Ok(Point2::new(ref_point.x + len, ref_point.y))
                } else {
                    Err(ParseError::MissingValue(
                        "Reference point required for length-only input".to_string(),
                    ))
                }
            }
            _ => Err(ParseError::InvalidFormat(
                "Input cannot be converted to point".to_string(),
            )),
        }
    }

    /// 解析为尺寸（宽高）
    pub fn parse_dimensions(input: &str) -> Result<(f64, f64), ParseError> {
        let input = input.trim();
        if let Some(comma_pos) = input.find(',') {
            let (w_str, h_str) = input.split_at(comma_pos);
            let h_str = &h_str[1..];

            let width = w_str
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidFormat(format!("Invalid width: {}", w_str)))?;
            let height = h_str
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidFormat(format!("Invalid height: {}", h_str)))?;

            Ok((width, height))
        } else {
            Err(ParseError::InvalidFormat(
                "Dimensions must be in format 'width,height'".to_string(),
            ))
        }
    }

    /// 将极坐标转换为点
    fn polar_to_point(origin: Point2, distance: f64, angle: f64) -> Point2 {
        Point2::new(
            origin.x + distance * angle.cos(),
            origin.y + distance * angle.sin(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_absolute_coordinate() {
        let result = InputParser::parse("100,50", None).unwrap();
        assert!(matches!(result, InputValue::Point(p) if p.x == 100.0 && p.y == 50.0));
    }

    #[test]
    fn test_parse_relative_coordinate() {
        let ref_point = Point2::new(10.0, 20.0);
        let result = InputParser::parse("@100,50", Some(ref_point)).unwrap();
        assert!(matches!(result, InputValue::Point(p) if p.x == 110.0 && p.y == 70.0));
    }

    #[test]
    fn test_parse_polar_relative() {
        let ref_point = Point2::new(0.0, 0.0);
        let result = InputParser::parse("@100<45", Some(ref_point)).unwrap();
        match result {
            InputValue::Point(p) => {
                let expected_x = 100.0 * (45.0_f64.to_radians().cos());
                let expected_y = 100.0 * (45.0_f64.to_radians().sin());
                assert!((p.x - expected_x).abs() < 1e-10);
                assert!((p.y - expected_y).abs() < 1e-10);
            }
            _ => panic!("Expected Point"),
        }
    }

    #[test]
    fn test_parse_length_angle() {
        let result = InputParser::parse("100<45", None).unwrap();
        match result {
            InputValue::LengthAngle { length, angle } => {
                assert_eq!(length, 100.0);
                assert!((angle - 45.0_f64.to_radians()).abs() < 1e-10);
            }
            _ => panic!("Expected LengthAngle"),
        }
    }

    #[test]
    fn test_parse_length() {
        let result = InputParser::parse("100", None).unwrap();
        assert!(matches!(result, InputValue::Length(100.0)));
    }

    #[test]
    fn test_parse_angle() {
        let result = InputParser::parse("<45", None).unwrap();
        assert!(matches!(result, InputValue::Angle(a) if (a - 45.0_f64.to_radians()).abs() < 1e-10));
    }

    #[test]
    fn test_parse_dimensions() {
        let (w, h) = InputParser::parse_dimensions("100,50").unwrap();
        assert_eq!(w, 100.0);
        assert_eq!(h, 50.0);
    }
}
