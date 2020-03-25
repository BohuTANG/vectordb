// Copyright 2020 The VectorDB Authors.
//
// Code is licensed under Apache License, Version 2.0.

use crate::datums::Datum;
use crate::errors::Error;

use super::{Binary, Constant, Variable};

pub trait IExpression {
    fn eval(&self) -> Result<Datum, Error>;
}

pub enum Expression {
    ConstantExpression(Constant),
    VariableExpression(Variable),
    BinaryExpression(Binary),
}

impl IExpression for Expression {
    fn eval(&self) -> Result<Datum, Error> {
        match self {
            Expression::ConstantExpression(v) => v.eval(),
            Expression::VariableExpression(v) => v.eval(),
            Expression::BinaryExpression(v) => v.eval(),
        }
    }
}