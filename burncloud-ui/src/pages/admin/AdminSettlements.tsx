import React, { useState } from 'react';
import {
  BCPageHeader,
  BCCard,
  BCButton,
  BCBadge
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminSettlements() {
  const { t } = useTranslation();
  const [batches, setBatches] = useState([
    {
      id: 'BATCH-2026-08B',
      period: 'Aug 15 - Aug 31, 2026',
      totalSuppliers: 24,
      totalGpus: 420,
      totalTokens: '1,840.5',
      totalPayout: '$124,500.00',
      status: 'Pending Review'
    },
    {
      id: 'BATCH-2026-08A',
      period: 'Aug 01 - Aug 15, 2026',
      totalSuppliers: 24,
      totalGpus: 412,
      totalTokens: '1,790.0',
      totalPayout: '$121,200.00',
      status: 'Settled & Paid'
    }
  ]);

  const handleApproveBatch = (id: string) => {
    setBatches(batches.map(b => b.id === id ? { ...b, status: 'Settled & Paid' } : b));
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.settlements.title}
        subtitle={t.admin.settlements.subtitle}
        conclusion={{
          text: t.admin.settlements.conclusion,
          type: 'healthy'
        }}
      />

      {/* Batches */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.settlements.batchesTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.settlements.batchesSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.settlements.colBatchId}</th>
                <th className="pb-3 font-semibold">{t.admin.settlements.colBillingCycle}</th>
                <th className="pb-3 font-semibold">{t.admin.settlements.colActiveSuppliers}</th>
                <th className="pb-3 font-semibold">{t.admin.settlements.colInferenceTokens}</th>
                <th className="pb-3 font-semibold">{t.admin.settlements.colTotalPayout}</th>
                <th className="pb-3 font-semibold">{t.admin.settlements.colStatus}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.settlements.colAction}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {batches.map((b) => (
                <tr key={b.id} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-bold text-gray-900">{b.id}</td>
                  <td className="py-3.5 text-gray-600 font-sans">{b.period}</td>
                  <td className="py-3.5 text-gray-700">{b.totalSuppliers} Providers ({b.totalGpus} GPUs)</td>
                  <td className="py-3.5 text-gray-700">{b.totalTokens}M</td>
                  <td className="py-3.5 font-bold text-gray-950">{b.totalPayout}</td>
                  <td className="py-3.5 font-sans">
                    <BCBadge variant={b.status === 'Settled & Paid' ? 'success' : 'warning'} size="sm">
                      {b.status}
                    </BCBadge>
                  </td>
                  <td className="py-3.5 text-right font-sans">
                    {b.status === 'Pending Review' ? (
                      <BCButton
                        variant="primary"
                        size="xs"
                        onClick={() => handleApproveBatch(b.id)}
                      >
                        {t.admin.settlements.btnApprove}
                      </BCButton>
                    ) : (
                      <span className="text-xs text-gray-400 font-mono">Completed</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>
    </div>
  );
}
