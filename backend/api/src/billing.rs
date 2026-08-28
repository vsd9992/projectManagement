use rust_decimal::Decimal;

/// The computed tax figures for one invoice's base (taxable) amount. Kept
/// separate from retention, which is a contractual withholding, not a tax.
pub struct TaxBreakdown {
    pub gst_amount: Decimal,
    pub gst_tds_amount: Decimal,
}

/// A pluggable regional tax/billing rule set — the abstraction behind the
/// generic billing engine decision
/// (.ai/decisions/current/2026-08-27-generic-billing-engine-india-first-profile.md).
/// Only `IndiaGstProfile` exists today; a tenant's `region_profile` column
/// selects which one applies, so adding a second region later doesn't touch
/// this trait or its callers.
pub trait RegionProfile {
    fn compute_tax(&self, base_amount: Decimal) -> TaxBreakdown;
}

/// GST works-contract SAC 9954 at 18%, GST TDS at 2% — both computed on the
/// taxable value (base_amount), not the tax-inclusive amount, matching how
/// GST TDS is actually calculated under Section 51 CGST Act. Mobilization-
/// advance recovery is not implemented yet — it needs a running per-project
/// advance balance that doesn't exist as a tracked entity yet, and M5's
/// verification doesn't require it; deferred, not silently dropped.
pub struct IndiaGstProfile;

impl RegionProfile for IndiaGstProfile {
    fn compute_tax(&self, base_amount: Decimal) -> TaxBreakdown {
        TaxBreakdown {
            gst_amount: base_amount * Decimal::new(18, 2),
            gst_tds_amount: base_amount * Decimal::new(2, 2),
        }
    }
}

/// Resolves a tenant's `region_profile` string to its implementation.
/// Currently always India — the only profile built — regardless of the
/// input, since there is nothing else to fall back to yet. This function is
/// the single place that will need to change when a second profile exists.
pub fn profile_for(_region_profile: &str) -> Box<dyn RegionProfile + Send> {
    Box::new(IndiaGstProfile)
}

/// The full computed figures for a milestone-based invoice.
pub struct InvoiceCalculation {
    pub gst_amount: Decimal,
    pub gst_tds_amount: Decimal,
    pub retention_amount: Decimal,
    pub gross_amount: Decimal,
    pub net_payable: Decimal,
}

/// Computes an invoice's figures from its base (taxable) amount and a
/// per-invoice retention percentage:
/// - gross_amount = base_amount + gst_amount (the tax-inclusive invoice value)
/// - net_payable  = gross_amount - gst_tds_amount - retention_amount
pub fn calculate_invoice(
    profile: &dyn RegionProfile,
    base_amount: Decimal,
    retention_percent: Decimal,
) -> InvoiceCalculation {
    let tax = profile.compute_tax(base_amount);
    let retention_amount = base_amount * (retention_percent / Decimal::from(100));
    let gross_amount = base_amount + tax.gst_amount;
    let net_payable = gross_amount - tax.gst_tds_amount - retention_amount;

    InvoiceCalculation {
        gst_amount: tax.gst_amount,
        gst_tds_amount: tax.gst_tds_amount,
        retention_amount,
        gross_amount,
        net_payable,
    }
}
