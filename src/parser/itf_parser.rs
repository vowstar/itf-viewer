// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::*;
use crate::parser::lexer::*;
use nom::{
    branch::alt,
    character::complete::multispace0,
    combinator::{opt, value},
    number::complete::double,
    sequence::{preceded, tuple},
    IResult,
};
use std::collections::HashMap;

pub struct ItfParser {}

impl ItfParser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_itf_file(&mut self, content: &str) -> Result<ProcessStack, ParseError> {
        // Skip lexical analysis for now to get basic parsing working
        // let mut lexer = ItfLexer::new(content);
        // let _tokens = lexer.tokenize()
        //     .map_err(|e| ParseError::LexError(format!("{e:?}")))?;

        let (remaining, technology_info) = self.parse_header(content)
            .map_err(|e| ParseError::ParseError(format!("Header parse error: {e:?}")))?;

        let mut stack = ProcessStack::new(technology_info);
        let mut remaining = remaining;

        while !remaining.trim().is_empty() {
            // Skip empty lines and comments
            let trimmed = remaining.trim_start();
            if trimmed.is_empty() || trimmed.starts_with("$") {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
                continue;
            }
            
            if let Ok((rest, layer)) = self.parse_dielectric_layer(remaining) {
                stack.add_layer(Layer::Dielectric(layer));
                remaining = rest;
            } else if let Ok((rest, layer)) = self.parse_conductor_layer(remaining) {
                stack.add_layer(Layer::Conductor(Box::new(layer)));
                remaining = rest;
            } else if let Ok((rest, via)) = self.parse_via(remaining) {
                stack.add_via(via);
                remaining = rest;
            } else if let Ok((rest, temp)) = preceded(
                tuple((multispace0, parse_keyword("GLOBAL_TEMPERATURE"), parse_equals)),
                preceded(multispace0, double),
            )(remaining) {
                stack.technology_info.global_temperature = Some(temp);
                remaining = rest;
            } else if let Ok((rest, direction)) = preceded(
                tuple((multispace0, parse_keyword("REFERENCE_DIRECTION"), parse_equals)),
                preceded(multispace0, parse_identifier),
            )(remaining) {
                stack.technology_info.reference_direction = Some(direction);
                remaining = rest;
            } else if let Ok((rest, er)) = preceded(
                tuple((multispace0, parse_keyword("BACKGROUND_ER"), parse_equals)),
                preceded(multispace0, double),
            )(remaining) {
                stack.technology_info.background_er = Some(er);
                remaining = rest;
            } else {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                let skipped_line = &remaining[..next_line_end];
                if !skipped_line.trim().is_empty() && !skipped_line.trim().starts_with("$") {
                    eprintln!("Warning: Skipping unrecognized line: {}", skipped_line.trim());
                }
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
            }
        }

        stack.validate_stack()
            .map_err(|e| ParseError::ValidationError(format!("{e}")))?;

        Ok(stack)
    }

    fn parse_header<'a>(&self, input: &'a str) -> IResult<&'a str, TechnologyInfo> {
        let (input, technology_name) = preceded(
            tuple((parse_keyword("TECHNOLOGY"), parse_equals)),
            preceded(multispace0, parse_identifier),
        )(input)?;

        let mut tech_info = TechnologyInfo::new(technology_name);
        let mut remaining = input;

        while !remaining.trim().is_empty() {
            // Skip comments and empty lines in header
            let trimmed = remaining.trim_start();
            if trimmed.is_empty() || trimmed.starts_with("$") {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
                continue;
            }
            
            if let Ok((rest, temp)) = preceded(
                tuple((multispace0, parse_keyword("GLOBAL_TEMPERATURE"), parse_equals)),
                preceded(multispace0, double),
            )(remaining)
            {
                tech_info.global_temperature = Some(temp);
                remaining = rest;
            } else if let Ok((rest, direction)) = preceded(
                tuple((multispace0, parse_keyword("REFERENCE_DIRECTION"), parse_equals)),
                preceded(multispace0, parse_identifier),
            )(remaining)
            {
                tech_info.reference_direction = Some(direction);
                remaining = rest;
            } else if let Ok((rest, er)) = preceded(
                tuple((multispace0, parse_keyword("BACKGROUND_ER"), parse_equals)),
                preceded(multispace0, double),
            )(remaining)
            {
                tech_info.background_er = Some(er);
                remaining = rest;
            } else if let Ok((rest, factor)) = preceded(
                tuple((multispace0, parse_keyword("HALF_NODE_SCALE_FACTOR"), parse_equals)),
                preceded(multispace0, double),
            )(remaining)
            {
                tech_info.half_node_scale_factor = Some(factor);
                remaining = rest;
            } else if let Ok((rest, use_si)) = preceded(
                tuple((multispace0, parse_keyword("USE_SI_DENSITY"), parse_equals)),
                preceded(multispace0, alt((
                    value(true, parse_keyword("YES")),
                    value(false, parse_keyword("NO")),
                ))),
            )(remaining)
            {
                tech_info.use_si_density = Some(use_si);
                remaining = rest;
            } else if let Ok((rest, drop_factor)) = preceded(
                tuple((multispace0, parse_keyword("DROP_FACTOR_LATERAL_SPACING"), parse_equals)),
                preceded(multispace0, double),
            )(remaining)
            {
                tech_info.drop_factor_lateral_spacing = Some(drop_factor);
                remaining = rest;
            } else {
                break;
            }
        }

        Ok((remaining, tech_info))
    }

    fn parse_dielectric_layer<'a>(&self, input: &'a str) -> IResult<&'a str, DielectricLayer> {
        let (input, (_, name, _)) = tuple((
            preceded(multispace0, parse_keyword("DIELECTRIC")),
            preceded(multispace0, parse_identifier),
            preceded(multispace0, parse_left_brace),
        ))(input)?;

        let mut layer = DielectricLayer::new(name, 0.0, 0.0);
        let (input, properties) = self.parse_dielectric_properties(input)?;

        layer.thickness = properties.get("THICKNESS").copied().unwrap_or(0.0);
        layer.dielectric_constant = properties.get("ER").copied().unwrap_or(1.0);
        layer.measured_from = properties.get("MEASURED_FROM")
            .map(|_| "TOP_OF_CHIP".to_string());
        layer.sw_t = properties.get("SW_T").copied();
        layer.tw_t = properties.get("TW_T").copied();

        let (input, _) = preceded(multispace0, parse_right_brace)(input)?;

        Ok((input, layer))
    }

    fn parse_dielectric_properties<'a>(&self, input: &'a str) -> IResult<&'a str, HashMap<String, f64>> {
        let mut properties = HashMap::new();
        let mut remaining = input;

        while !remaining.trim_start().starts_with('}') && !remaining.trim().is_empty() {
            if let Ok((rest, (prop_name, _, value))) = tuple((
                preceded(multispace0, parse_identifier),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                properties.insert(prop_name.to_uppercase(), value);
                remaining = rest;
            } else if let Ok((rest, prop_name)) = preceded(
                multispace0,
                parse_identifier,
            )(remaining)
            {
                if prop_name.to_uppercase() == "MEASURED_FROM" {
                    if let Ok((rest2, _)) = preceded(
                        tuple((multispace0, parse_equals, multispace0)),
                        parse_identifier,
                    )(rest)
                    {
                        properties.insert("MEASURED_FROM".to_string(), 1.0);
                        remaining = rest2;
                    } else {
                        remaining = rest;
                    }
                } else {
                    remaining = rest;
                }
            } else {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
            }
        }

        Ok((remaining, properties))
    }

    fn parse_conductor_layer<'a>(&self, input: &'a str) -> IResult<&'a str, ConductorLayer> {
        let (input, (_, name, _)) = tuple((
            preceded(multispace0, parse_keyword("CONDUCTOR")),
            preceded(multispace0, parse_identifier),
            preceded(multispace0, parse_left_brace),
        ))(input)?;

        let mut layer = ConductorLayer::new(name, 0.0);
        let (input, _) = self.parse_conductor_properties(input, &mut layer)?;
        let (input, _) = preceded(multispace0, parse_right_brace)(input)?;

        Ok((input, layer))
    }

    fn parse_conductor_properties<'a>(&self, input: &'a str, layer: &mut ConductorLayer) -> IResult<&'a str, ()> {
        let mut remaining = input;

        while !remaining.trim_start().starts_with('}') && !remaining.trim().is_empty() {
            if let Ok((rest, (_, _, thickness))) = tuple((
                preceded(multispace0, parse_keyword("THICKNESS")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                layer.thickness = thickness;
                layer.physical_props.thickness = thickness;
                remaining = rest;
            } else if let Ok((rest, (_, _, crt1))) = tuple((
                preceded(multispace0, parse_keyword("CRT1")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                layer.electrical_props.crt1 = Some(crt1);
                remaining = rest;
            } else if let Ok((rest, (_, _, crt2))) = tuple((
                preceded(multispace0, parse_keyword("CRT2")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                layer.electrical_props.crt2 = Some(crt2);
                remaining = rest;
            } else if let Ok((rest, (_, _, rpsq))) = tuple((
                preceded(multispace0, parse_keyword("RPSQ")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                layer.electrical_props.rpsq = Some(rpsq);
                remaining = rest;
            } else if let Ok((rest, (_, _, wmin))) = tuple((
                preceded(multispace0, parse_keyword("WMIN")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                layer.physical_props.width_min = Some(wmin);
                remaining = rest;
            } else if let Ok((rest, (_, _, smin))) = tuple((
                preceded(multispace0, parse_keyword("SMIN")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                layer.physical_props.spacing_min = Some(smin);
                remaining = rest;
            } else if let Ok((rest, (_, _, side_tangent))) = tuple((
                preceded(multispace0, parse_keyword("SIDE_TANGENT")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                layer.physical_props.side_tangent = Some(side_tangent);
                remaining = rest;
            } else if let Ok((rest, table)) = preceded(
                tuple((multispace0, parse_keyword("RHO_VS_WIDTH_AND_SPACING"))),
                |input| self.parse_lookup_table_2d(input),
            )(remaining)
            {
                layer.rho_vs_width_spacing = Some(table);
                remaining = rest;
            } else if let Ok((rest, table)) = preceded(
                tuple((multispace0, parse_keyword("ETCH_VS_WIDTH_AND_SPACING"))),
                |input| self.parse_etch_table(input),
            )(remaining)
            {
                layer.etch_vs_width_spacing = Some(table);
                remaining = rest;
            } else if let Ok((rest, table)) = preceded(
                tuple((multispace0, parse_keyword("THICKNESS_VS_WIDTH_AND_SPACING"))),
                |input| self.parse_lookup_table_2d(input),
            )(remaining)
            {
                layer.thickness_vs_width_spacing = Some(table);
                remaining = rest;
            } else if let Ok((rest, _)) = preceded(
                tuple((multispace0, parse_keyword("POLYNOMIAL_BASED_THICKNESS_VARIATION"))),
                |input| self.skip_complex_block(input),
            )(remaining)
            {
                remaining = rest;
            } else if let Ok((rest, _)) = preceded(
                tuple((multispace0, parse_keyword("RHO_VS_SI_WIDTH_AND_THICKNESS"))),
                |input| self.skip_complex_block(input),
            )(remaining)
            {
                remaining = rest;
            } else if let Ok((rest, _)) = preceded(
                tuple((multispace0, parse_keyword("CRT_VS_SI_WIDTH"))),
                |input| self.skip_complex_block(input),
            )(remaining)
            {
                remaining = rest;
            } else {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
            }
        }

        Ok((remaining, ()))
    }

    fn parse_lookup_table_2d<'a>(&self, input: &'a str) -> IResult<&'a str, LookupTable2D> {
        let (input, _) = preceded(multispace0, parse_left_brace)(input)?;
        
        let (input, widths) = preceded(
            tuple((multispace0, parse_keyword("WIDTHS"))),
            parse_number_list,
        )(input)?;

        let (input, spacings) = preceded(
            tuple((multispace0, parse_keyword("SPACINGS"))),
            parse_number_list,
        )(input)?;

        let (input, values) = preceded(
            tuple((multispace0, parse_keyword("VALUES"))),
            parse_2d_number_matrix,
        )(input)?;

        let (input, _) = preceded(multispace0, parse_right_brace)(input)?;

        Ok((input, LookupTable2D::new(widths, spacings, values)))
    }

    fn parse_etch_table<'a>(&self, input: &'a str) -> IResult<&'a str, LookupTable2D> {
        let (input, _) = opt(preceded(
            multispace0,
            parse_identifier, // Parse optional modifiers like "ETCH_FROM_TOP", "CAPACITIVE_ONLY", etc.
        ))(input)?;

        self.parse_lookup_table_2d(input)
    }

    fn skip_complex_block<'a>(&self, input: &'a str) -> IResult<&'a str, ()> {
        let mut remaining = input;
        let mut brace_count = 0;
        let mut in_brace = false;

        // Skip to the first '{'
        while !remaining.is_empty() {
            if remaining.starts_with('{') {
                brace_count = 1;
                in_brace = true;
                remaining = &remaining[1..];
                break;
            } else if remaining.starts_with('\n') {
                remaining = &remaining[1..];
            } else {
                remaining = &remaining[1..];
            }
        }

        if !in_brace {
            return Ok((remaining, ()));
        }

        // Skip until we match all braces
        while !remaining.is_empty() && brace_count > 0 {
            if remaining.starts_with('{') {
                brace_count += 1;
                remaining = &remaining[1..];
            } else if remaining.starts_with('}') {
                brace_count -= 1;
                remaining = &remaining[1..];
            } else {
                remaining = &remaining[1..];
            }
        }

        Ok((remaining, ()))
    }

    fn parse_via<'a>(&self, input: &'a str) -> IResult<&'a str, ViaConnection> {
        let (input, (_, name, _)) = tuple((
            preceded(multispace0, parse_keyword("VIA")),
            preceded(multispace0, parse_identifier),
            preceded(multispace0, parse_left_brace),
        ))(input)?;

        let mut from_layer = String::new();
        let mut to_layer = String::new();
        let mut area = 0.0;
        let mut rpv = 0.0;
        let mut remaining = input;

        while !remaining.trim_start().starts_with('}') && !remaining.trim().is_empty() {
            if let Ok((rest, (_, _, layer_name))) = tuple((
                preceded(multispace0, parse_keyword("FROM")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, parse_identifier),
            ))(remaining)
            {
                from_layer = layer_name;
                remaining = rest;
            } else if let Ok((rest, (_, _, layer_name))) = tuple((
                preceded(multispace0, parse_keyword("TO")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, parse_identifier),
            ))(remaining)
            {
                to_layer = layer_name;
                remaining = rest;
            } else if let Ok((rest, (_, _, area_val))) = tuple((
                preceded(multispace0, parse_keyword("AREA")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                area = area_val;
                remaining = rest;
            } else if let Ok((rest, (_, _, rpv_val))) = tuple((
                preceded(multispace0, parse_keyword("RPV")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            ))(remaining)
            {
                rpv = rpv_val;
                remaining = rest;
            } else {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
            }
        }

        let (input, _) = preceded(multispace0, parse_right_brace)(remaining)?;

        Ok((input, ViaConnection::new(name, from_layer, to_layer, area, rpv)))
    }
}

impl Default for ItfParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Lexical analysis error: {0}")]
    LexError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

pub fn parse_itf_file(content: &str) -> Result<ProcessStack, ParseError> {
    let mut parser = ItfParser::new();
    parser.parse_itf_file(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_itf() {
        let itf_content = r#"
TECHNOLOGY = test_tech
GLOBAL_TEMPERATURE = 25.0

DIELECTRIC oxide1 {THICKNESS=1.0 ER=4.2}
CONDUCTOR metal1 {THICKNESS=0.5 CRT1=3.5e-3 CRT2=-1.0e-7 RPSQ=0.05 WMIN=0.1 SMIN=0.1}
DIELECTRIC oxide2 {THICKNESS=2.0 ER=4.2}

VIA via1 { FROM=metal1 TO=oxide2 AREA=0.04 RPV=5.0 }
        "#;

        let result = parse_itf_file(itf_content);
        assert!(result.is_ok());
        
        let stack = result.unwrap();
        assert_eq!(stack.technology_info.name, "test_tech");
        assert_eq!(stack.technology_info.global_temperature, Some(25.0));
        assert_eq!(stack.get_layer_count(), 3);
        assert_eq!(stack.get_via_count(), 1);
    }

    #[test]
    fn test_parse_dielectric_layer() {
        let parser = ItfParser::new();
        let input = "DIELECTRIC test_oxide {THICKNESS=1.5 ER=3.9 MEASURED_FROM=TOP_OF_CHIP}";
        
        let result = parser.parse_dielectric_layer(input);
        assert!(result.is_ok());
        
        let (_, layer) = result.unwrap();
        assert_eq!(layer.name, "test_oxide");
        assert_eq!(layer.thickness, 1.5);
        assert_eq!(layer.dielectric_constant, 3.9);
    }

    #[test]
    fn test_parse_conductor_layer() {
        let parser = ItfParser::new();
        let input = "CONDUCTOR test_metal {THICKNESS=0.8 CRT1=2.5e-3 SIDE_TANGENT=0.05 WMIN=0.2 SMIN=0.15}";
        
        let result = parser.parse_conductor_layer(input);
        assert!(result.is_ok());
        
        let (_, layer) = result.unwrap();
        assert_eq!(layer.name, "test_metal");
        assert_eq!(layer.thickness, 0.8);
        assert_eq!(layer.electrical_props.crt1, Some(2.5e-3));
        assert_eq!(layer.physical_props.side_tangent, Some(0.05));
        assert_eq!(layer.physical_props.width_min, Some(0.2));
        assert_eq!(layer.physical_props.spacing_min, Some(0.15));
    }

    #[test]
    fn test_parse_via() {
        let parser = ItfParser::new();
        let input = "VIA test_via { FROM=layer1 TO=layer2 AREA=0.025 RPV=10.0 }";
        
        let result = parser.parse_via(input);
        assert!(result.is_ok());
        
        let (_, via) = result.unwrap();
        assert_eq!(via.name, "test_via");
        assert_eq!(via.from_layer, "layer1");
        assert_eq!(via.to_layer, "layer2");
        assert_eq!(via.area, 0.025);
        assert_eq!(via.resistance_per_via, 10.0);
    }

    #[test]
    fn test_parse_header() {
        let parser = ItfParser::new();
        let input = r#"TECHNOLOGY = advanced_tech
GLOBAL_TEMPERATURE = 85.0
REFERENCE_DIRECTION = VERTICAL
BACKGROUND_ER = 3.0"#;
        
        let result = parser.parse_header(input);
        assert!(result.is_ok());
        
        let (_, tech_info) = result.unwrap();
        assert_eq!(tech_info.name, "advanced_tech");
        assert_eq!(tech_info.global_temperature, Some(85.0));
        assert_eq!(tech_info.reference_direction, Some("VERTICAL".to_string()));
        assert_eq!(tech_info.background_er, Some(3.0));
    }
}