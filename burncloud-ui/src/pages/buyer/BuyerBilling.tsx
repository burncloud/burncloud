import React, { useState } from 'react';
import {
  Plus,
  Download,
  CheckCircle2,
  ShieldCheck,
  Zap
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCButton,
  BCBadge,
  BCModal,
  BCInput
} from '@/components/ui';
import { useRole } from '@/context/RoleContext';
import { useTranslation } from '@/i18n/I18nContext';

export function BuyerBilling() {
  const { balance, setBalance, todaySpend } = useRole();
  const { t } = useTranslation();
  const [isTopUpModalOpen, setIsTopUpModalOpen] = useState(false);
  const [selectedTopUpAmount, setSelectedTopUpAmount] = useState<number>(100);
  const [customAmount, setCustomAmount] = useState<string>('');
  const [autoRechargeEnabled, setAutoRechargeEnabled] = useState(true);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const handleTopUp = (e: React.FormEvent) => {
    e.preventDefault();
    const amount = customAmount ? parseFloat(customAmount) : selectedTopUpAmount;
    if (isNaN(amount) || amount <= 0) return;

    setBalance((prev) => prev + amount);
    setIsTopUpModalOpen(false);
    setSuccessMessage(`Successfully recharged $${amount.toFixed(2)} to your prepaid balance.`);
    setTimeout(() => setSuccessMessage(null), 4000);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.buyer.billing.title}
        subtitle={t.buyer.billing.subtitle}
        conclusion={{
          text: `Current balance: $${balance.toFixed(2)}. At ~$${todaySpend.toFixed(2)}/day, you have ~${Math.floor(balance / Math.max(0.1, todaySpend))} days of runtime.`,
          type: balance < 20 ? 'warning' : 'healthy'
        }}
        actions={
          <BCButton
            variant="primary"
            size="sm"
            onClick={() => setIsTopUpModalOpen(true)}
          >
            <Plus className="w-3.5 h-3.5" />
            <span>{t.buyer.billing.topUpBtn}</span>
          </BCButton>
        }
      />

      {successMessage && (
        <div className="p-4 bg-emerald-50 border border-emerald-200 rounded-2xl flex items-center gap-3 text-xs font-medium text-emerald-900">
          <CheckCircle2 className="w-4 h-4 text-emerald-600 flex-shrink-0" />
          <span>{successMessage}</span>
        </div>
      )}

      {/* Balance Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
        <BCCard className="p-6 space-y-3 bg-gradient-to-br from-gray-900 to-gray-950 text-white border-0">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-mono text-gray-400 uppercase tracking-wider">
              {t.buyer.billing.prepaidBalance}
            </span>
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="text-3xl font-bold font-mono tracking-tight text-white">
            ${balance.toFixed(2)}
          </div>
          <div className="text-xs text-gray-400 pt-2 border-t border-gray-800 flex items-center justify-between">
            <span>Burn Rate: ~${todaySpend.toFixed(2)} / day</span>
            <button
              onClick={() => setIsTopUpModalOpen(true)}
              className="text-xs font-bold text-white hover:underline flex items-center gap-1 cursor-pointer"
            >
              + {t.buyer.billing.topUpBtn}
            </button>
          </div>
        </BCCard>

        <BCCard className="p-6 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-mono text-gray-400 uppercase tracking-wider">
              {t.buyer.billing.autoRecharge}
            </span>
            <BCBadge variant={autoRechargeEnabled ? 'success' : 'neutral'} size="sm">
              {autoRechargeEnabled ? 'ENABLED' : 'DISABLED'}
            </BCBadge>
          </div>
          <div className="text-sm font-bold text-gray-900">
            Auto top up $100.00 when balance falls below $20.00
          </div>
          <div className="pt-2 border-t border-gray-100 flex items-center justify-between text-xs text-gray-500">
            <span>Payment: Visa ending in 4242</span>
            <button
              onClick={() => setAutoRechargeEnabled(!autoRechargeEnabled)}
              className="text-xs font-semibold text-blue-600 hover:underline cursor-pointer"
            >
              Toggle
            </button>
          </div>
        </BCCard>

        <BCCard className="p-6 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-mono text-gray-400 uppercase tracking-wider">
              {t.buyer.billing.meteringPolicy}
            </span>
            <Zap className="w-4 h-4 text-amber-500" />
          </div>
          <div className="text-xs text-gray-700 leading-relaxed font-sans">
            {t.buyer.billing.meteringDesc}
          </div>
          <div className="pt-2 border-t border-gray-100 text-[11px] font-mono text-emerald-700">
            ✓ 0% Platform markup on frontier models
          </div>
        </BCCard>
      </div>

      {/* Transaction History & Receipts */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.buyer.billing.invoicesTitle}</h3>
            <p className="text-xs text-gray-500">{t.buyer.billing.invoicesDesc}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 font-mono uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.buyer.billing.colInvoiceId}</th>
                <th className="pb-3 font-semibold">{t.buyer.billing.colDate}</th>
                <th className="pb-3 font-semibold">{t.buyer.billing.colAmount}</th>
                <th className="pb-3 font-semibold">Payment Method</th>
                <th className="pb-3 font-semibold">{t.buyer.billing.colAmount}</th>
                <th className="pb-3 font-semibold">{t.buyer.billing.colStatus}</th>
                <th className="pb-3 font-semibold text-right">{t.buyer.billing.colReceipt}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 font-mono">
              {[
                {
                  id: 'INV-2026-0882',
                  date: '2026-08-20',
                  desc: 'Prepaid Token Balance Top Up',
                  method: 'Visa •••• 4242',
                  amount: '$100.00',
                  status: 'Paid'
                },
                {
                  id: 'INV-2026-0741',
                  date: '2026-08-01',
                  desc: 'Prepaid Token Balance Top Up',
                  method: 'Visa •••• 4242',
                  amount: '$200.00',
                  status: 'Paid'
                },
                {
                  id: 'INV-2026-0610',
                  date: '2026-07-15',
                  desc: 'Prepaid Token Balance Top Up',
                  method: 'Corporate Wire (ACH)',
                  amount: '$500.00',
                  status: 'Paid'
                }
              ].map((inv) => (
                <tr key={inv.id} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-bold text-gray-900">{inv.id}</td>
                  <td className="py-3.5 text-gray-600">{inv.date}</td>
                  <td className="py-3.5 font-sans text-gray-900">{inv.desc}</td>
                  <td className="py-3.5 text-gray-600">{inv.method}</td>
                  <td className="py-3.5 font-bold text-gray-950">{inv.amount}</td>
                  <td className="py-3.5">
                    <BCBadge variant="success" size="sm">{inv.status}</BCBadge>
                  </td>
                  <td className="py-3.5 text-right">
                    <button
                      onClick={() => alert(`Downloading PDF statement for ${inv.id}`)}
                      className="text-blue-600 hover:text-blue-800 font-semibold hover:underline flex items-center gap-1 ml-auto font-sans cursor-pointer"
                    >
                      <Download className="w-3 h-3" />
                      <span>PDF</span>
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>

      {/* Top Up Modal */}
      <BCModal
        isOpen={isTopUpModalOpen}
        onClose={() => setIsTopUpModalOpen(false)}
        title={t.buyer.billing.modalTitle}
        subtitle={t.buyer.billing.modalSubtitle}
      >
        <form onSubmit={handleTopUp} className="space-y-4">
          <div className="space-y-2">
            <label className="text-xs font-semibold text-gray-700">{t.buyer.billing.selectAmount}</label>
            <div className="grid grid-cols-3 gap-2">
              {[50, 100, 250, 500, 1000].map((amt) => (
                <button
                  key={amt}
                  type="button"
                  onClick={() => {
                    setSelectedTopUpAmount(amt);
                    setCustomAmount('');
                  }}
                  className={`py-2.5 rounded-xl border text-xs font-mono font-bold transition-all cursor-pointer ${
                    selectedTopUpAmount === amt && !customAmount
                      ? 'bg-gray-900 text-white border-gray-900 shadow-xs'
                      : 'bg-white text-gray-800 border-gray-200 hover:bg-gray-50'
                  }`}
                >
                  ${amt}
                </button>
              ))}
            </div>
          </div>

          <div className="space-y-1.5 pt-2">
            <label className="text-xs font-semibold text-gray-700">{t.buyer.billing.customAmount}</label>
            <BCInput
              type="number"
              placeholder="e.g. 1500"
              value={customAmount}
              onChange={(e) => setCustomAmount(e.target.value)}
              min={10}
            />
          </div>

          <div className="p-3 bg-gray-50 rounded-xl border border-gray-200/80 text-xs text-gray-600 space-y-1">
            <div className="flex justify-between">
              <span>{t.buyer.billing.paymentMethod}:</span>
              <span className="font-semibold text-gray-900">Visa •••• 4242</span>
            </div>
            <div className="flex justify-between">
              <span>{t.buyer.billing.instantCredit}:</span>
              <span className="font-bold text-emerald-700 font-mono">
                ${customAmount ? parseFloat(customAmount || '0').toFixed(2) : selectedTopUpAmount.toFixed(2)}
              </span>
            </div>
          </div>

          <div className="pt-3 border-t border-gray-100 flex items-center justify-end gap-2">
            <BCButton
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => setIsTopUpModalOpen(false)}
            >
              {t.common.cancel}
            </BCButton>
            <BCButton type="submit" variant="primary" size="sm">
              {t.buyer.billing.confirmRecharge}
            </BCButton>
          </div>
        </form>
      </BCModal>
    </div>
  );
}
