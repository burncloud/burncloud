use super::model::SupplierEarningsModel;

fn escape_csv(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn to_csv(model: &SupplierEarningsModel) -> String {
    let mut csv = String::from("Node,GPU Hardware,Model,Tokens,Revenue Share,Earnings\r\n");

    for row in model.rows {
        csv.push_str(
            &[
                row.node,
                row.hardware,
                row.model,
                row.tokens,
                row.revenue_share,
                row.earnings,
            ]
            .into_iter()
            .map(escape_csv)
            .collect::<Vec<_>>()
            .join(","),
        );
        csv.push_str("\r\n");
    }

    csv
}

pub fn download_csv(csv: &str) {
    let Ok(serialized) = serde_json::to_string(csv) else {
        return;
    };

    let script = format!(
        "const csv = {serialized}; \
         const blob = new Blob([\"\\uFEFF\", csv], {{ type: \"text/csv;charset=utf-8;\" }}); \
         const url = URL.createObjectURL(blob); \
         const link = document.createElement(\"a\"); \
         link.href = url; \
         link.download = \"supplier-earnings.csv\"; \
         document.body.appendChild(link); \
         link.click(); \
         link.remove(); \
         URL.revokeObjectURL(url);"
    );
    dioxus::document::eval(&script);
}
