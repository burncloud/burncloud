use super::{actions::to_csv, model::SupplierEarningsModel};
use crate::app::router::routes::Route;

#[test]
fn reference_earnings_match_supplier_page() {
    let model = SupplierEarningsModel::reference();

    assert_eq!(model.rows.len(), 4);
    assert_eq!(model.today_earnings, "$382.40");
    assert_eq!(model.monthly_earnings, "$8,420.50");
    assert_eq!(model.total_tokens, "482.1M");
    assert_eq!(model.effective_rate, "$1.94");
    assert_eq!(model.rows[0].hardware, "8x H100 SXM5");
    assert_eq!(model.rows[0].revenue_share, "80%");
    assert_eq!(model.rows[3].earnings, "$18.20");
}

#[test]
fn earnings_csv_contains_header_and_all_reference_rows() {
    let csv = to_csv(SupplierEarningsModel::reference());

    assert!(csv.starts_with("Node,GPU Hardware,Model,Tokens,Revenue Share,Earnings\r\n"));
    assert_eq!(csv.lines().count(), 5);
    assert!(csv.contains("\"DeepSeek V3 (671B)\""));
    assert!(csv.contains("\"$184.20\""));
    assert!(csv.contains("\"75%\""));
}

#[test]
fn supplier_earnings_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/supplier/earnings".parse::<Route>(),
        Ok(Route::SupplierEarnings {})
    ));
}
