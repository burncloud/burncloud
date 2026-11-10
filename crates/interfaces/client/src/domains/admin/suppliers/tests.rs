use super::model::SUPPLIERS;
use crate::app::router::routes::Route;

#[test]
fn reference_supplier_directory_matches_the_admin_page() {
    assert_eq!(SUPPLIERS.len(), 4);
    assert_eq!(
        SUPPLIERS.map(|supplier| (
            supplier.name,
            supplier.tier,
            supplier.connected_gpus,
            supplier.uptime_sla,
            supplier.revenue_share,
            supplier.earnings_30d,
            supplier.status
        )),
        [
            (
                "Silicon Bay Data Centers LLC",
                "L4 Strategic",
                "128x H100 SXM5",
                "99.98%",
                "85%",
                "$118,400",
                "Online"
            ),
            (
                "Frankfurt EuroCompute GmbH",
                "L3 Professional",
                "64x A100 80GB",
                "99.94%",
                "80%",
                "$52,100",
                "Online"
            ),
            (
                "Tokyo Cloud Matrix Ltd",
                "L3 Professional",
                "48x H100 SXM5",
                "99.91%",
                "80%",
                "$44,800",
                "Online"
            ),
            (
                "Nordic AI Infra AS",
                "L2 Verified",
                "24x RTX 4090 Pool",
                "99.72%",
                "75%",
                "$12,400",
                "Online"
            ),
        ]
    );
}

#[test]
fn admin_suppliers_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/admin/suppliers".parse::<Route>(),
        Ok(Route::AdminSuppliers {})
    ));
}
