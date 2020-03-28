// Copyright 2020 The VectorDB Authors.
//
// Code is licensed under Apache License, Version 2.0.

use sqlparser::ast::{Expr, Query, SetExpr, Statement, TableFactor, Value as ExprValue};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::datums::Datum;
use crate::errors::{Error, SQLError};

use super::{
    BinaryExpressionPlanner, ConstantPlanner, MapPlanner, Planner, Planner::*,
    ScalarExpressionPlanner, SourcePlanner, VariablePlanner,
};

pub fn parser(sql: String) -> Result<Statement, Error> {
    let dialect = GenericDialect {};

    let mut parsed = match Parser::parse_sql(&dialect, sql) {
        Ok(v) => v,
        Err(e) => return Err(Error::SQL(SQLError::ParserError(e))),
    };

    let ast = match parsed.pop() {
        Some(v) => v,
        None => return Err(Error::SQL(SQLError::UnsupportedOperation)),
    };

    Ok(ast)
}

pub fn handle_statement(stmt: Statement) -> Result<Planner, Error> {
    match stmt {
        Statement::Query(query) => handle_query(*query),
        _ => Err(Error::SQL(SQLError::UnsupportedOperation)),
    }
}

pub fn handle_query(query: Query) -> Result<Planner, Error> {
    let sqlparser::ast::Query { body, .. } = query;

    let (mut from, selection) = match body {
        SetExpr::Select(select) => (select.from, select.selection),
        _ => return Err(Error::SQL(SQLError::UnsupportedOperation)),
    };

    // Source Planner.
    let table = from.pop().map(|t| t.relation);
    let source = handle_source_planner(table)?;

    // Filter Planner.
    let filter = match selection {
        Some(ref expr) => handle_expression_planner(&expr)?,
        None => Null,
    };

    let mut planners = MapPlanner::new();
    planners.planners.push(source);
    planners.planners.push(filter);
    Ok(Map(planners))
}

pub fn handle_source_planner(relation: Option<TableFactor>) -> Result<Planner, Error> {
    let object_name = match relation {
        Some(TableFactor::Table { name, .. }) => name,
        Some(e) => {
            return Err(Error::SQL(SQLError::NotImplemented(format!(
                "Table: {}",
                e
            ))))
        }
        None => return Err(Error::SQL(SQLError::UnsupportedOperation)),
    };

    let (schema, table) = match object_name.0.len() {
        1 => ("", object_name.0.get(0).unwrap().as_str()),
        2 => (
            object_name.0.get(0).unwrap().as_str(),
            object_name.0.get(1).unwrap().as_str(),
        ),
        _ => {
            return Err(Error::SQL(SQLError::NotImplemented(format!(
                "{:?}",
                object_name.0
            ))))
        }
    };

    Ok(Planner::from(SourcePlanner::new(
        schema.to_string(),
        table.to_string(),
    )))
}

pub fn handle_expression_planner(expr: &Expr) -> Result<Planner, Error> {
    match expr {
        // Variable.
        Expr::Identifier(ref identifier) => Ok(Planner::from(VariablePlanner::new(identifier))),

        // Constant.
        Expr::Value(ref val) => Ok(Planner::from(ConstantPlanner::new(
            expression_value_to_datum(val)?,
        ))),

        // Binary.
        Expr::BinaryOp {
            ref left,
            ref op,
            ref right,
        } => {
            let left_expression_planner = handle_expression_planner(left)?;
            let right_expression_planner = handle_expression_planner(right)?;
            Ok(Planner::from(BinaryExpressionPlanner::new(
                format!("{}", op),
                left_expression_planner,
                right_expression_planner,
            )))
        }

        // Function.
        Expr::Function(func) => {
            let mut arguments: Vec<Planner> = Vec::new();
            for arg in &func.args {
                let argument = handle_expression_planner(&arg)?;
                arguments.push(argument);
            }
            Ok(Planner::from(ScalarExpressionPlanner::new(
                format!("{}", func.name),
                arguments,
            )))
        }

        // Unsupported.
        _ => Err(Error::SQL(SQLError::NotImplemented(format!("{:?}", expr)))),
    }
}

pub fn expression_value_to_datum(val: &ExprValue) -> Result<Datum, Error> {
    match val {
        // Number.
        ExprValue::Number(v) => {
            let i = v.parse::<i64>().unwrap();
            Ok(Datum::Int64(i))
        }

        // String.
        ExprValue::SingleQuotedString(ref v) => Ok(Datum::String(v.to_string())),

        // Unsupported.
        _ => Err(Error::SQL(SQLError::NotImplemented(format!("{:?}", val)))),
    }
}

#[test]
fn test_parser() {
    {
        let sql = "";
        let query = parser(sql.to_string());
        assert_eq!(true, query.is_err());
    }

    {
        let sql = "SELECT a, b FROM table_1 WHERE a > b AND b < 100 OR myfunc(a+1) = 1";
        let stmt = parser(sql.to_string());
        assert_eq!(true, stmt.is_ok());
        print!("{:#?}", stmt);
        let planner = handle_statement(stmt.unwrap());
        print!("{:#?}", planner);
        assert_eq!(true, planner.is_ok());

        print!("{:#?}", planner.unwrap());
    }
}
