// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, digit1, multispace0},
    combinator::{map, opt, recognize, value},
    multi::{separated_list0, separated_list1},
    number::complete::double,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Keyword(String),
    Identifier(String),
    Number(f64),
    String(String),
    LeftBrace,
    RightBrace,
    Equals,
    Comment(String),
    Newline,
    EOF,
}

#[derive(Debug, Clone)]
pub struct ItfLexer<'a> {
    input: &'a str,
}

impl<'a> ItfLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        let mut remaining = self.input;

        while !remaining.is_empty() {
            match self.next_token(remaining) {
                Ok((rest, token)) => {
                    if !matches!(token, Token::Comment(_) | Token::Newline) {
                        tokens.push(token);
                    }
                    remaining = rest;
                }
                Err(e) => return Err(LexError::ParseError(format!("Lexing error: {e:?}"))),
            }
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }

    fn next_token(&self, input: &'a str) -> IResult<&'a str, Token> {
        alt((
            |i| self.parse_comment(i),
            |i| self.parse_whitespace(i),
            |i| self.parse_number(i),
            |i| self.parse_keyword_or_identifier(i),
            |i| self.parse_string(i),
            |i| self.parse_symbol(i),
        ))(input)
    }

    fn parse_comment(&self, input: &'a str) -> IResult<&'a str, Token> {
        alt((
            // Parse $$ comments
            preceded(
                tag("$$"),
                map(
                    terminated(take_until("\n"), opt(char('\n'))),
                    |comment: &str| Token::Comment(comment.trim().to_string()),
                ),
            ),
            // Parse $ comments (single $)
            preceded(
                tag("$"),
                map(
                    terminated(take_until("\n"), opt(char('\n'))),
                    |comment: &str| Token::Comment(comment.trim().to_string()),
                ),
            ),
        ))(input)
    }

    fn parse_whitespace(&self, input: &'a str) -> IResult<&'a str, Token> {
        alt((
            value(Token::Newline, char('\n')),
            value(Token::Newline, tag("\r\n")),
            map(
                take_while1(|c: char| c.is_whitespace() && c != '\n'),
                |_| Token::Newline,
            ),
        ))(input)
    }

    fn parse_keyword_or_identifier(&self, input: &'a str) -> IResult<&'a str, Token> {
        map(
            take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '+' || c == '-'),
            |s: &str| {
                let upper_s = s.to_uppercase();
                if self.is_keyword(&upper_s) {
                    Token::Keyword(upper_s)
                } else {
                    Token::Identifier(s.to_string())
                }
            },
        )(input)
    }

    fn parse_number(&self, input: &'a str) -> IResult<&'a str, Token> {
        map(
            recognize(tuple((
                opt(alt((char('+'), char('-')))),
                alt((
                    recognize(tuple((digit1, char('.'), opt(digit1)))),
                    recognize(tuple((opt(digit1), char('.'), digit1))),
                    digit1,
                )),
                opt(tuple((
                    alt((char('e'), char('E'))),
                    opt(alt((char('+'), char('-')))),
                    digit1,
                ))),
            ))),
            |num_str: &str| {
                Token::Number(num_str.parse::<f64>().unwrap_or(0.0))
            },
        )(input)
    }

    fn parse_string(&self, input: &'a str) -> IResult<&'a str, Token> {
        alt((
            delimited(
                char('"'),
                map(take_until("\""), |s: &str| Token::String(s.to_string())),
                char('"'),
            ),
            delimited(
                char('\''),
                map(take_until("'"), |s: &str| Token::String(s.to_string())),
                char('\''),
            ),
        ))(input)
    }

    fn parse_symbol(&self, input: &'a str) -> IResult<&'a str, Token> {
        alt((
            value(Token::LeftBrace, char('{')),
            value(Token::RightBrace, char('}')),
            value(Token::Equals, char('=')),
        ))(input)
    }

    fn is_keyword(&self, word: &str) -> bool {
        matches!(
            word,
            "TECHNOLOGY"
                | "GLOBAL_TEMPERATURE"
                | "REFERENCE_DIRECTION"
                | "BACKGROUND_ER"
                | "HALF_NODE_SCALE_FACTOR"
                | "USE_SI_DENSITY"
                | "DROP_FACTOR_LATERAL_SPACING"
                | "DIELECTRIC"
                | "CONDUCTOR"
                | "VIA"
                | "THICKNESS"
                | "ER"
                | "CRT1"
                | "CRT2"
                | "RPSQ"
                | "WMIN"
                | "SMIN"
                | "SIDE_TANGENT"
                | "RHO_VS_WIDTH_AND_SPACING"
                | "ETCH_VS_WIDTH_AND_SPACING"
                | "THICKNESS_VS_WIDTH_AND_SPACING"
                | "POLYNOMIAL_BASED_THICKNESS_VARIATION"
                | "DENSITY_POLYNOMIAL_ORDERS"
                | "WIDTH_POLYNOMIAL_ORDERS"
                | "WIDTH_RANGES"
                | "POLYNOMIAL_COEFFICIENTS"
                | "RHO_VS_SI_WIDTH_AND_THICKNESS"
                | "CRT_VS_SI_WIDTH"
                | "WIDTHS"
                | "SPACINGS"
                | "VALUES"
                | "FROM"
                | "TO"
                | "AREA"
                | "RPV"
                | "MEASURED_FROM"
                | "TOP_OF_CHIP"
                | "ETCH_FROM_TOP"
                | "CAPACITIVE_ONLY"
                | "RESISTIVE_ONLY"
                | "VERTICAL"
                | "HORIZONTAL"
                | "GATE"
                | "YES"
                | "NO"
                | "SW_T"
                | "TW_T"
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LexError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Invalid number format: {0}")]
    InvalidNumber(String),
    
    #[error("Unexpected character: {0}")]
    UnexpectedCharacter(char),
}

pub fn parse_number_list(input: &str) -> IResult<&str, Vec<f64>> {
    use nom::character::complete::{char as nom_char, space1};
    
    preceded(
        multispace0,
        delimited(
            nom_char('{'),
            preceded(
                multispace0,
                separated_list0(
                    space1,
                    preceded(multispace0, double),
                ),
            ),
            preceded(multispace0, nom_char('}')),
        ),
    )(input)
}

pub fn parse_2d_number_matrix(input: &str) -> IResult<&str, Vec<Vec<f64>>> {
    use nom::character::complete::{line_ending, space1};
    preceded(
        multispace0,
        delimited(
            char('{'),
            preceded(
                multispace0,
                separated_list0(
                    line_ending,
                    separated_list1(
                        space1,
                        double,
                    ),
                ),
            ),
            preceded(multispace0, char('}')),
        ),
    )(input)
}

pub fn parse_identifier(input: &str) -> IResult<&str, String> {
    preceded(
        multispace0,
        map(
            take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '+' || c == '-'),
            |s: &str| s.to_string(),
        ),
    )(input)
}

pub fn parse_keyword(keyword: &str) -> impl Fn(&str) -> IResult<&str, ()> + '_ {
    move |input: &str| {
        preceded(
            multispace0,
            value((), tag(keyword)),
        )(input)
    }
}

pub fn parse_equals(input: &str) -> IResult<&str, ()> {
    preceded(multispace0, value((), char('=')))(input)
}

pub fn parse_left_brace(input: &str) -> IResult<&str, ()> {
    preceded(multispace0, value((), char('{')))(input)
}

pub fn parse_right_brace(input: &str) -> IResult<&str, ()> {
    preceded(multispace0, value((), char('}')))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let mut lexer = ItfLexer::new("TECHNOLOGY = test_tech");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::Keyword("TECHNOLOGY".to_string()));
        assert_eq!(tokens[1], Token::Equals);
        assert_eq!(tokens[2], Token::Identifier("test_tech".to_string()));
    }

    #[test]
    fn test_tokenize_numbers() {
        let mut lexer = ItfLexer::new("1.5 -2.3E-4 0.123");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::Number(1.5));
        assert_eq!(tokens[1], Token::Number(-2.3e-4));
        assert_eq!(tokens[2], Token::Number(0.123));
    }

    #[test]
    fn test_tokenize_braces() {
        let mut lexer = ItfLexer::new("DIELECTRIC oxide { THICKNESS=1.0 }");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::Keyword("DIELECTRIC".to_string()));
        assert_eq!(tokens[1], Token::Identifier("oxide".to_string()));
        assert_eq!(tokens[2], Token::LeftBrace);
        assert_eq!(tokens[3], Token::Keyword("THICKNESS".to_string()));
        assert_eq!(tokens[4], Token::Equals);
        assert_eq!(tokens[5], Token::Number(1.0));
        assert_eq!(tokens[6], Token::RightBrace);
    }

    #[test]
    fn test_parse_number_list() {
        let input = "{ 1.0 2.0 3.0 }";
        let (_, numbers) = parse_number_list(input).unwrap();
        assert_eq!(numbers, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_parse_2d_matrix() {
        let input = "{ 1.0 2.0\n3.0 4.0 }";
        let (_, matrix) = parse_2d_number_matrix(input).unwrap();
        assert_eq!(matrix, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn test_comments() {
        let mut lexer = ItfLexer::new("TECHNOLOGY = test $$ This is a comment\nTHICKNESS = 1.0");
        let tokens = lexer.tokenize().unwrap();
        
        let non_comment_tokens: Vec<_> = tokens.into_iter()
            .filter(|t| !matches!(t, Token::Comment(_)))
            .collect();
        
        assert_eq!(non_comment_tokens[0], Token::Keyword("TECHNOLOGY".to_string()));
        assert_eq!(non_comment_tokens[1], Token::Equals);
        assert_eq!(non_comment_tokens[2], Token::Identifier("test".to_string()));
    }
}