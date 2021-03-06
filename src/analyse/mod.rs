use crate::broker_statement::BrokerStatement;
use crate::commissions::CommissionCalc;
use crate::config::{Config, PortfolioConfig};
use crate::core::{GenericResult, EmptyResult};
use crate::currency::converter::CurrencyConverter;
use crate::db;
use crate::quotes::Quotes;

use self::performance::PortfolioPerformanceAnalyser;

pub mod deposit_emulator;
mod performance;
mod sell_simulation;

pub fn analyse(config: &Config, portfolio_name: &str, show_closed_positions: bool) -> EmptyResult {
    let (portfolio, mut statement, converter, mut quotes) = load(config, portfolio_name)?;
    let mut commission_calc = CommissionCalc::new(statement.broker.commission_spec.clone());

    statement.check_date();
    statement.batch_quotes(&mut quotes);

    for (symbol, &quantity) in statement.open_positions.clone().iter() {
        statement.emulate_sell(&symbol, quantity, quotes.get(&symbol)?, &mut commission_calc)?;
    }
    statement.process_trades()?;
    statement.emulate_commissions(commission_calc);
    statement.merge_symbols(&portfolio.merge_performance).map_err(|e| format!(
        "Invalid performance merging configuration: {}", e))?;

    for currency in ["USD", "RUB"].iter() {
        PortfolioPerformanceAnalyser::analyse(
            &statement, &portfolio, *currency, &converter, show_closed_positions)?;
    }

    Ok(())
}

pub fn simulate_sell(config: &Config, portfolio_name: &str, positions: &[(String, Option<u32>)]) -> EmptyResult {
    let (portfolio, statement, converter, quotes) = load(config, portfolio_name)?;
    sell_simulation::simulate_sell(portfolio, statement, &converter, quotes, positions)
}

fn load<'a>(config: &'a Config, portfolio_name: &str) -> GenericResult<
    (&'a PortfolioConfig, BrokerStatement, CurrencyConverter, Quotes)
> {
    let portfolio = config.get_portfolio(portfolio_name)?;
    let statement = BrokerStatement::read(config, portfolio.broker, &portfolio.statements)?;

    let database = db::connect(&config.db_path)?;
    let converter = CurrencyConverter::new(database.clone(), false);
    let quotes = Quotes::new(&config, database)?;

    Ok((portfolio, statement, converter, quotes))
}