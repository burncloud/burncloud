import React, { useState } from 'react';
import {
  DollarSign,
  Download,
  Building2,
  CheckCircle2
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCButton,
  BCBadge,
  BCModal
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function SupplierSettlements() {
  const { t } = useTranslation();
  const [isWithdrawModalOpen, setIsWithdrawModalOpen] = useState(false);
  const [success, setSuccess] = useState(false);

  const handleWithdraw = () => {
    setSuccess(true);
    setIsWithdrawModalOpen(false);
    setTimeout(() => setSuccess(false), 4000);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.supplier.settlements.title}
        subtitle={t.supplier.settlements.subtitle}
        conclusion={{
          text: t.supplier.settlements.conclusion,
          type: 'healthy'
        }}
        actions={
          <BCButton
            variant="primary"
            size="sm"
            onClick={() => setIsWithdrawModalOpen(true)}
          >
            <DollarSign className="w-3.5 h-3.5" />
            <span>{t.supplier.settlements.requestPayout}</span>
          </BCButton>
        }
      />

      {success && (
        <div className="p-4 bg-emerald-50 border border-emerald-200 rounded-2xl flex items-center gap-3 text-xs font-medium text-emerald-900">
          <CheckCircle2 className="w-4 h-4 text-emerald-600 flex-shrink-0" />
          <span>Payout request of $6,420.00 submitted. Funds will arrive within 1-2 business days.</span>
        </div>
      )}

      {/* Payout Summary */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
        <BCCard className="p-6 space-y-3 bg-gradient-to-br from-gray-900 to-gray-950 text-white border-0">
          <span className="text-[11px] font-mono text-gray-400 uppercase tracking-wider">
            {t.supplier.settlements.settledReady}
          </span>
          <div className="text-3xl font-bold font-mono text-white">$6,420.00</div>
          <div className="text-xs text-gray-400 pt-2 border-t border-gray-800 flex justify-between">
            <span>Next Auto-Payout: Sept 1</span>
            <button
              onClick={() => setIsWithdrawModalOpen(true)}
              className="text-xs font-bold text-white hover:underline cursor-pointer"
            >
              {t.supplier.settlements.withdrawNow} →
            </button>
          </div>
        </BCCard>

        <BCCard className="p-6 space-y-3">
          <span className="text-[11px] font-mono text-gray-400 uppercase tracking-wider">
            {t.supplier.settlements.primaryMethod}
          </span>
          <div className="flex items-center gap-2">
            <Building2 className="w-5 h-5 text-gray-700" />
            <div className="text-sm font-bold text-gray-900">Silicon Valley Bank (ACH)</div>
          </div>
          <div className="text-xs text-gray-500 pt-2 border-t border-gray-100 flex justify-between">
            <span>Account ending in •••• 9102</span>
            <span className="text-emerald-700 font-bold font-mono">VERIFIED</span>
          </div>
        </BCCard>

        <BCCard className="p-6 space-y-3">
          <span className="text-[11px] font-mono text-gray-400 uppercase tracking-wider">
            {t.supplier.settlements.totalHistorical}
          </span>
          <div className="text-2xl font-bold font-mono text-gray-950">$38,910.40</div>
          <div className="text-xs text-gray-500 pt-2 border-t border-gray-100 flex justify-between">
            <span>12 Completed Settlement Batches</span>
            <span className="text-gray-400">0 Disputes</span>
          </div>
        </BCCard>
      </div>

      {/* Payout History Ledger */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.supplier.settlements.ledgerTitle}</h3>
            <p className="text-xs text-gray-500">{t.supplier.settlements.ledgerSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.supplier.settlements.colBatch}</th>
                <th className="pb-3 font-semibold">{t.supplier.settlements.colDate}</th>
                <th className="pb-3 font-semibold">{t.supplier.settlements.colDestination}</th>
                <th className="pb-3 font-semibold">{t.supplier.settlements.colTokens}</th>
                <th className="pb-3 font-semibold">{t.supplier.settlements.colNetPayout}</th>
                <th className="pb-3 font-semibold">{t.supplier.settlements.colStatus}</th>
                <th className="pb-3 font-semibold text-right">{t.supplier.settlements.colStatement}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  id: 'SET-2026-0815',
                  date: '2026-08-15',
                  dest: 'ACH •••• 9102',
                  tokens: '512.4M',
                  amount: '$7,840.00',
                  status: 'Completed'
                },
                {
                  id: 'SET-2026-0801',
                  date: '2026-08-01',
                  dest: 'ACH •••• 9102',
                  tokens: '490.1M',
                  amount: '$7,120.50',
                  status: 'Completed'
                },
                {
                  id: 'SET-2026-0715',
                  date: '2026-07-15',
                  dest: 'USDC (0x9a8f...3d11)',
                  tokens: '620.0M',
                  amount: '$8,940.00',
                  status: 'Completed'
                }
              ].map((item) => (
                <tr key={item.id} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-bold text-gray-900">{item.id}</td>
                  <td className="py-3.5 text-gray-600">{item.date}</td>
                  <td className="py-3.5 text-gray-700 font-sans">{item.dest}</td>
                  <td className="py-3.5 text-gray-700">{item.tokens}</td>
                  <td className="py-3.5 font-bold text-gray-950">{item.amount}</td>
                  <td className="py-3.5 font-sans">
                    <BCBadge variant="success" size="sm">{item.status}</BCBadge>
                  </td>
                  <td className="py-3.5 text-right">
                    <button
                      onClick={() => alert(`Downloading statement ${item.id}`)}
                      className="text-blue-600 hover:text-blue-800 font-sans font-medium hover:underline flex items-center gap-1 ml-auto cursor-pointer"
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

      {/* Modal: Withdraw */}
      <BCModal
        isOpen={isWithdrawModalOpen}
        onClose={() => setIsWithdrawModalOpen(false)}
        title={t.supplier.settlements.modalTitle}
        subtitle={t.supplier.settlements.modalSubtitle}
      >
        <div className="space-y-4 text-xs font-sans">
          <div className="p-4 bg-gray-50 rounded-xl border border-gray-200/80 space-y-2 font-mono">
            <div className="flex justify-between">
              <span className="text-gray-500">Available Balance:</span>
              <span className="font-bold text-gray-900">$6,420.00</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-500">Payout Destination:</span>
              <span className="font-bold text-gray-900">Silicon Valley Bank (•••• 9102)</span>
            </div>
            <div className="flex justify-between pt-2 border-t border-gray-200">
              <span className="text-gray-900 font-bold">Transfer Total:</span>
              <span className="font-bold text-emerald-700 text-sm">$6,420.00</span>
            </div>
          </div>

          <div className="pt-2 flex justify-end gap-2">
            <BCButton
              variant="secondary"
              size="sm"
              onClick={() => setIsWithdrawModalOpen(false)}
            >
              {t.common.cancel}
            </BCButton>
            <BCButton
              variant="primary"
              size="sm"
              onClick={handleWithdraw}
            >
              {t.supplier.settlements.modalConfirm}
            </BCButton>
          </div>
        </div>
      </BCModal>
    </div>
  );
}
