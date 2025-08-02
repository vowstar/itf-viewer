// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::*;
use crate::parser::lexer::*;
use nom::{
    branch::alt,
    character::complete::multispace0,
    combinator::{opt, value},
    number::complete::double,
    sequence::preceded,
    IResult, Parser,
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

        let (remaining, technology_info) = self
            .parse_header(content)
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
                (
                    multispace0,
                    parse_keyword("GLOBAL_TEMPERATURE"),
                    parse_equals,
                ),
                preceded(multispace0, double),
            )
            .parse(remaining)
            {
                stack.technology_info.global_temperature = Some(temp);
                remaining = rest;
            } else if let Ok((rest, direction)) = preceded(
                (
                    multispace0,
                    parse_keyword("REFERENCE_DIRECTION"),
                    parse_equals,
                ),
                preceded(multispace0, parse_identifier),
            )
            .parse(remaining)
            {
                stack.technology_info.reference_direction = Some(direction);
                remaining = rest;
            } else if let Ok((rest, er)) = preceded(
                (multispace0, parse_keyword("BACKGROUND_ER"), parse_equals),
                preceded(multispace0, double),
            )
            .parse(remaining)
            {
                stack.technology_info.background_er = Some(er);
                remaining = rest;
            } else if let Ok((rest, table)) =
                preceded((multispace0, parse_keyword("CRT_VS_SI_WIDTH")), |input| {
                    self.parse_crt_vs_si_width_table(input)
                })
                .parse(remaining)
            {
                // Associate CRT_VS_SI_WIDTH table with the most recent conductor layer
                if let Some(Layer::Conductor(conductor)) = stack.layers.last_mut() {
                    conductor.crt_vs_si_width = Some(table);
                    println!(
                        "INFO: Associated CRT_VS_SI_WIDTH table with conductor '{}'",
                        conductor.name
                    );
                }
                remaining = rest;
            } else {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                let skipped_line = &remaining[..next_line_end];
                if !skipped_line.trim().is_empty() && !skipped_line.trim().starts_with("$") {
                    eprintln!("WARN: Skipping unrecognized line: {}", skipped_line.trim());
                }
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
            }
        }

        // Auto-create missing layers before validation
        stack.ensure_via_layers_exist();

        // Try strict validation first
        match stack.validate_stack_strict() {
            Ok(()) => {
                // Strict validation passed
            }
            Err(_) => {
                // Strict validation failed, try lenient validation
                match stack.validate_stack_lenient() {
                    Ok(warnings) => {
                        // Print warnings for missing layer references but continue
                        for warning in warnings {
                            eprintln!("WARN: {warning}");
                        }
                    }
                    Err(e) => {
                        // Even lenient validation failed - this is a serious error
                        return Err(ParseError::ValidationError(format!("{e}")));
                    }
                }
            }
        }

        Ok(stack)
    }

    fn parse_header<'a>(&self, input: &'a str) -> IResult<&'a str, TechnologyInfo> {
        let mut remaining = input;
        let mut tech_name: Option<String> = None;
        let mut global_temperature: Option<f64> = None;
        let mut reference_direction: Option<String> = None;
        let mut background_er: Option<f64> = None;
        let mut half_node_scale_factor: Option<f64> = None;
        let mut use_si_density: Option<bool> = None;
        let mut drop_factor_lateral_spacing: Option<f64> = None;

        // Parse header fields in any order until we hit a CONDUCTOR, DIELECTRIC, or VIA
        while !remaining.trim().is_empty() {
            let trimmed = remaining.trim_start();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("$") {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
                continue;
            }

            // Stop parsing header when we encounter layer definitions
            if trimmed.starts_with("CONDUCTOR")
                || trimmed.starts_with("DIELECTRIC")
                || trimmed.starts_with("VIA")
            {
                break;
            }

            // Try to parse various header fields
            if let Ok((rest, name)) = preceded(
                (parse_keyword("TECHNOLOGY"), parse_equals),
                preceded(multispace0, parse_identifier),
            )
            .parse(remaining)
            {
                tech_name = Some(name);
                remaining = rest;
            } else if let Ok((rest, temp)) = preceded(
                (
                    multispace0,
                    parse_keyword("GLOBAL_TEMPERATURE"),
                    parse_equals,
                ),
                preceded(multispace0, double),
            )
            .parse(remaining)
            {
                global_temperature = Some(temp);
                remaining = rest;
            } else if let Ok((rest, direction)) = preceded(
                (
                    multispace0,
                    parse_keyword("REFERENCE_DIRECTION"),
                    parse_equals,
                ),
                preceded(multispace0, parse_identifier),
            )
            .parse(remaining)
            {
                reference_direction = Some(direction);
                remaining = rest;
            } else if let Ok((rest, er)) = preceded(
                (multispace0, parse_keyword("BACKGROUND_ER"), parse_equals),
                preceded(multispace0, double),
            )
            .parse(remaining)
            {
                background_er = Some(er);
                remaining = rest;
            } else if let Ok((rest, factor)) = preceded(
                (
                    multispace0,
                    parse_keyword("HALF_NODE_SCALE_FACTOR"),
                    parse_equals,
                ),
                preceded(multispace0, double),
            )
            .parse(remaining)
            {
                half_node_scale_factor = Some(factor);
                remaining = rest;
            } else if let Ok((rest, use_si)) = preceded(
                (multispace0, parse_keyword("USE_SI_DENSITY"), parse_equals),
                preceded(
                    multispace0,
                    alt((
                        value(true, parse_keyword("YES")),
                        value(false, parse_keyword("NO")),
                    )),
                ),
            )
            .parse(remaining)
            {
                use_si_density = Some(use_si);
                remaining = rest;
            } else if let Ok((rest, drop_factor)) = preceded(
                (
                    multispace0,
                    parse_keyword("DROP_FACTOR_LATERAL_SPACING"),
                    parse_equals,
                ),
                preceded(multispace0, double),
            )
            .parse(remaining)
            {
                drop_factor_lateral_spacing = Some(drop_factor);
                remaining = rest;
            } else {
                // If we can't parse this line as a header field, skip it
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                let skipped_line = &remaining[..next_line_end];
                if !skipped_line.trim().is_empty() && !skipped_line.trim().starts_with("$") {
                    eprintln!(
                        "WARN: Skipping unrecognized header line: {}",
                        skipped_line.trim()
                    );
                }
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
            }
        }

        // TECHNOLOGY is required, use default if not found
        let technology_name = tech_name.unwrap_or_else(|| "unknown_technology".to_string());

        let mut tech_info = TechnologyInfo::new(technology_name);
        tech_info.global_temperature = global_temperature;
        tech_info.reference_direction = reference_direction;
        tech_info.background_er = background_er;
        tech_info.half_node_scale_factor = half_node_scale_factor;
        tech_info.use_si_density = use_si_density;
        tech_info.drop_factor_lateral_spacing = drop_factor_lateral_spacing;

        Ok((remaining, tech_info))
    }

    fn parse_dielectric_layer<'a>(&self, input: &'a str) -> IResult<&'a str, DielectricLayer> {
        let (input, (_, name, _)) = (
            preceded(multispace0, parse_keyword("DIELECTRIC")),
            preceded(multispace0, parse_identifier),
            preceded(multispace0, parse_left_brace),
        )
            .parse(input)?;

        let mut layer = DielectricLayer::new(name, 0.0, 0.0);
        let (input, properties) = self.parse_dielectric_properties(input)?;

        layer.thickness = properties.get("THICKNESS").copied().unwrap_or(0.0);
        layer.dielectric_constant = properties.get("ER").copied().unwrap_or(1.0);
        layer.measured_from = properties
            .get("MEASURED_FROM")
            .map(|_| "TOP_OF_CHIP".to_string());
        layer.sw_t = properties.get("SW_T").copied();
        layer.tw_t = properties.get("TW_T").copied();

        let (input, _) = preceded(multispace0, parse_right_brace).parse(input)?;

        Ok((input, layer))
    }

    fn parse_dielectric_properties<'a>(
        &self,
        input: &'a str,
    ) -> IResult<&'a str, HashMap<String, f64>> {
        let mut properties = HashMap::new();
        let mut remaining = input;

        while !remaining.trim_start().starts_with('}') && !remaining.trim().is_empty() {
            if let Ok((rest, (prop_name, _, value))) = (
                preceded(multispace0, parse_identifier),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                properties.insert(prop_name.to_uppercase(), value);
                remaining = rest;
            } else if let Ok((rest, prop_name)) =
                preceded(multispace0, parse_identifier).parse(remaining)
            {
                if prop_name.to_uppercase() == "MEASURED_FROM" {
                    if let Ok((rest2, _)) =
                        preceded((multispace0, parse_equals, multispace0), parse_identifier)
                            .parse(rest)
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
        let (input, (_, name, _)) = (
            preceded(multispace0, parse_keyword("CONDUCTOR")),
            preceded(multispace0, parse_identifier),
            preceded(multispace0, parse_left_brace),
        )
            .parse(input)?;

        let mut layer = ConductorLayer::new(name, 0.0);
        let (input, _) = self.parse_conductor_properties(input, &mut layer)?;
        let (input, _) = preceded(multispace0, parse_right_brace).parse(input)?;

        Ok((input, layer))
    }

    fn parse_conductor_properties<'a>(
        &self,
        input: &'a str,
        layer: &mut ConductorLayer,
    ) -> IResult<&'a str, ()> {
        let mut remaining = input;

        while !remaining.trim_start().starts_with('}') && !remaining.trim().is_empty() {
            if let Ok((rest, (_, _, thickness))) = (
                preceded(multispace0, parse_keyword("THICKNESS")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                layer.thickness = thickness;
                layer.physical_props.thickness = thickness;
                remaining = rest;
            } else if let Ok((rest, (_, _, crt1))) = (
                preceded(multispace0, parse_keyword("CRT1")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                layer.electrical_props.crt1 = Some(crt1);
                remaining = rest;
            } else if let Ok((rest, (_, _, crt2))) = (
                preceded(multispace0, parse_keyword("CRT2")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                layer.electrical_props.crt2 = Some(crt2);
                remaining = rest;
            } else if let Ok((rest, (_, _, rpsq))) = (
                preceded(multispace0, parse_keyword("RPSQ")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                layer.electrical_props.rpsq = Some(rpsq);
                remaining = rest;
            } else if let Ok((rest, (_, _, wmin))) = (
                preceded(multispace0, parse_keyword("WMIN")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                layer.physical_props.width_min = Some(wmin);
                remaining = rest;
            } else if let Ok((rest, (_, _, smin))) = (
                preceded(multispace0, parse_keyword("SMIN")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                layer.physical_props.spacing_min = Some(smin);
                remaining = rest;
            } else if let Ok((rest, (_, _, side_tangent))) = (
                preceded(multispace0, parse_keyword("SIDE_TANGENT")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                layer.physical_props.side_tangent = Some(side_tangent);
                remaining = rest;
            } else if let Ok((rest, table)) = preceded(
                (multispace0, parse_keyword("RHO_VS_WIDTH_AND_SPACING")),
                |input| self.parse_lookup_table_2d(input),
            )
            .parse(remaining)
            {
                layer.rho_vs_width_spacing = Some(table);
                remaining = rest;
            } else if let Ok((rest, table)) = preceded(
                (multispace0, parse_keyword("ETCH_VS_WIDTH_AND_SPACING")),
                |input| self.parse_etch_table(input),
            )
            .parse(remaining)
            {
                layer.etch_vs_width_spacing = Some(table);
                remaining = rest;
            } else if let Ok((rest, table)) = preceded(
                (multispace0, parse_keyword("THICKNESS_VS_WIDTH_AND_SPACING")),
                |input| self.parse_lookup_table_2d(input),
            )
            .parse(remaining)
            {
                layer.thickness_vs_width_spacing = Some(table);
                remaining = rest;
            } else if let Ok((rest, _)) = preceded(
                (
                    multispace0,
                    parse_keyword("POLYNOMIAL_BASED_THICKNESS_VARIATION"),
                ),
                |input| self.skip_complex_block(input),
            )
            .parse(remaining)
            {
                remaining = rest;
            } else if let Ok((rest, table)) = preceded(
                (multispace0, parse_keyword("RHO_VS_SI_WIDTH_AND_THICKNESS")),
                |input| self.parse_rho_vs_si_width_thickness_table(input),
            )
            .parse(remaining)
            {
                layer.rho_vs_si_width_thickness = Some(table);
                remaining = rest;
            } else if let Ok((rest, table)) =
                preceded((multispace0, parse_keyword("CRT_VS_SI_WIDTH")), |input| {
                    self.parse_crt_vs_si_width_table(input)
                })
                .parse(remaining)
            {
                layer.crt_vs_si_width = Some(table);
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
        let (input, _) = preceded(multispace0, parse_left_brace).parse(input)?;

        let (input, widths) =
            preceded((multispace0, parse_keyword("WIDTHS")), parse_number_list).parse(input)?;

        let (input, spacings) =
            preceded((multispace0, parse_keyword("SPACINGS")), parse_number_list).parse(input)?;

        let (input, values) = preceded(
            (multispace0, parse_keyword("VALUES")),
            parse_2d_number_matrix,
        )
        .parse(input)?;

        let (input, _) = preceded(multispace0, parse_right_brace).parse(input)?;

        Ok((input, LookupTable2D::new(widths, spacings, values)))
    }

    fn parse_etch_table<'a>(&self, input: &'a str) -> IResult<&'a str, LookupTable2D> {
        let (input, _) = opt(preceded(
            multispace0,
            parse_identifier, // Parse optional modifiers like "ETCH_FROM_TOP", "CAPACITIVE_ONLY", etc.
        ))
        .parse(input)?;

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
        let (input, (_, name, _)) = (
            preceded(multispace0, parse_keyword("VIA")),
            preceded(multispace0, parse_identifier),
            preceded(multispace0, parse_left_brace),
        )
            .parse(input)?;

        let mut from_layer = String::new();
        let mut to_layer = String::new();
        let mut area = 0.0;
        let mut rpv = 0.0;
        let mut remaining = input;

        while !remaining.trim_start().starts_with('}') && !remaining.trim().is_empty() {
            if let Ok((rest, (_, _, layer_name))) = (
                preceded(multispace0, parse_keyword("FROM")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, parse_identifier),
            )
                .parse(remaining)
            {
                from_layer = layer_name;
                remaining = rest;
            } else if let Ok((rest, (_, _, layer_name))) = (
                preceded(multispace0, parse_keyword("TO")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, parse_identifier),
            )
                .parse(remaining)
            {
                to_layer = layer_name;
                remaining = rest;
            } else if let Ok((rest, (_, _, area_val))) = (
                preceded(multispace0, parse_keyword("AREA")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                area = area_val;
                remaining = rest;
            } else if let Ok((rest, (_, _, rpv_val))) = (
                preceded(multispace0, parse_keyword("RPV")),
                preceded(multispace0, parse_equals),
                preceded(multispace0, double),
            )
                .parse(remaining)
            {
                rpv = rpv_val;
                remaining = rest;
            } else {
                // Check if there's a closing brace on this line - if so, we should stop here
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                let current_line = &remaining[..next_line_end];

                if let Some(brace_pos) = current_line.find('}') {
                    // Found closing brace on this line - only skip content before the brace
                    remaining = &remaining[brace_pos..];
                    break; // Exit the parsing loop, let the main parser handle the closing brace
                } else {
                    // No closing brace, skip the entire line
                    remaining = &remaining[next_line_end..];
                    if remaining.starts_with('\n') {
                        remaining = &remaining[1..];
                    }
                }
            }
        }

        let (input, _) = preceded(multispace0, parse_right_brace).parse(remaining)?;

        Ok((
            input,
            ViaConnection::new(name, from_layer, to_layer, area, rpv),
        ))
    }

    fn parse_crt_vs_si_width_table<'a>(
        &self,
        input: &'a str,
    ) -> IResult<&'a str, CrtVsSiWidthTable> {
        let (input, _) = preceded(multispace0, parse_left_brace).parse(input)?;

        let mut widths = Vec::new();
        let mut crt1_values = Vec::new();
        let mut crt2_values = Vec::new();
        let mut remaining = input;

        // Parse tuples of the form (width, crt1, crt2)
        while !remaining.trim_start().starts_with('}') && !remaining.trim().is_empty() {
            // Skip comments and empty lines
            let trimmed = remaining.trim_start();
            if trimmed.is_empty() || trimmed.starts_with("$") {
                let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
                remaining = &remaining[next_line_end..];
                if remaining.starts_with('\n') {
                    remaining = &remaining[1..];
                }
                continue;
            }

            // Find opening parenthesis
            if let Some(open_paren) = remaining.find('(') {
                let after_paren = &remaining[open_paren + 1..];
                // Find closing parenthesis
                if let Some(close_paren) = after_paren.find(')') {
                    let tuple_content = &after_paren[..close_paren];
                    // Split by commas and parse numbers
                    let parts: Vec<&str> = tuple_content.split(',').collect();
                    if parts.len() == 3 {
                        if let (Ok(width), Ok(crt1), Ok(crt2)) = (
                            parts[0].trim().parse::<f64>(),
                            parts[1].trim().parse::<f64>(),
                            parts[2].trim().parse::<f64>(),
                        ) {
                            widths.push(width);
                            crt1_values.push(crt1);
                            crt2_values.push(crt2);
                            remaining = &after_paren[close_paren + 1..];
                            continue;
                        }
                    }
                }
            }

            // Skip this line if we can't parse it
            let next_line_end = remaining.find('\n').unwrap_or(remaining.len());
            remaining = &remaining[next_line_end..];
            if remaining.starts_with('\n') {
                remaining = &remaining[1..];
            }
        }

        let (input, _) = preceded(multispace0, parse_right_brace).parse(remaining)?;

        Ok((
            input,
            CrtVsSiWidthTable::new(widths, crt1_values, crt2_values),
        ))
    }

    fn parse_rho_vs_si_width_thickness_table<'a>(
        &self,
        input: &'a str,
    ) -> IResult<&'a str, LookupTable2D> {
        let (input, _) = preceded(multispace0, parse_left_brace).parse(input)?;

        let (input, widths) =
            preceded((multispace0, parse_keyword("WIDTH")), parse_number_list).parse(input)?;

        let (input, thicknesses) =
            preceded((multispace0, parse_keyword("THICKNESS")), parse_number_list).parse(input)?;

        let (input, values) = preceded(
            (multispace0, parse_keyword("VALUES")),
            parse_2d_number_matrix,
        )
        .parse(input)?;

        let (input, _) = preceded(multispace0, parse_right_brace).parse(input)?;

        Ok((input, LookupTable2D::new(widths, thicknesses, values)))
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
