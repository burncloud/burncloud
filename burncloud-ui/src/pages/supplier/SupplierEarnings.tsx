import React from 'react';
import {
  Download
} from 'lucide-react';
import {
  BCPageHeader,
  BCMetric,
  BCCard,
  BCButton
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function SupplierEarnings() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.supplier.earnings.title}
        subtitle={t.supplier.earnings.subtitle}
        conclusion={{
          text: t.supplier.earnings.conclusion,
          type: 'healthy'
        }}
        actions={
          <BCButton variant="secondary" size="sm">
            <Download className="w-3.5 h-3.5" />
            <span>{t.supplier.earnings.exportCsv}</span>
          </BCButton>
        }
      />

      {/* Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <BCMetric
          label={t.supplier.earnings.metricTodayNet}
          value="$382.40"
          trend="+14.2% vs yesterday"
          trendPositive={true}
        />
        <BCMetric
          label={t.supplier.earnings.metricMonthNet}
          value="$8,420.50"
          subtitle="Aug 1 - Aug 22"
        />
        <BCMetric
          label={t.supplier.earnings.metricTokensServed}
          value="482.1M"
          unit="tokens"
          subtitle="99.98% valid tokens"
        />
        <BCMetric
          label={t.supplier.earnings.metricAvgRateGpu}
          value="$1.94"
          subtitle="8x H100 effective rate"
        />
      </div>

      {/* Breakdown by Cluster & Model */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.supplier.earnings.breakdownTitle}</h3>
            <p className="text-xs text-gray-500">{t.supplier.earnings.breakdownSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.supplier.earnings.colNode}</th>
                <th className="pb-3 font-semibold">GPU Hardware</th>
                <th className="pb-3 font-semibold">{t.supplier.earnings.colModel}</th>
                <th className="pb-3 font-semibold">{t.supplier.earnings.colTokens}</th>
                <th className="pb-3 font-semibold">{t.supplier.earnings.colRevenueShare}</th>
                <th className="pb-3 font-semibold text-right">{t.supplier.earnings.colNetEarnings}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  node: 'SJC-Pod-01-Rack4',
                  hw: '8x H100 SXM5',
                  model: 'DeepSeek V3 (671B)',
                  tokens: '184.2M',
                  revShare: '80%',
                  accrual: '$184.20'
                },
                {
                  node: 'SJC-Pod-01-Rack5',
                  hw: '8x H100 SXM5',
                  model: 'DeepSeek R1',
                  tokens: '92.4M',
                  revShare: '80%',
                  accrual: '$178.60'
                },
                {
                  node: 'FRA-DC2-Compute-08',
                  hw: '8x A100 80GB',
                  model: 'Qwen 2.5 72B',
                  tokens: '142.1M',
                  revShare: '80%',
                  accrual: '$79.40'
                },
                {
                  node: 'HKG-Edge-RTX-Pool',
                  hw: '4x RTX 4090',
                  model: 'Llama 3.3 70B',
                  tokens: '63.4M',
                  revShare: '75%',
                  accrual: '$18.20'
                }
              ].map((item, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-bold text-gray-900 font-sans">{item.node}</td>
                  <td className="py-3.5 text-gray-700 font-sans">{item.hw}</td>
                  <td className="py-3.5 text-gray-700 font-sans">{item.model}</td>
                  <td className="py-3.5 text-gray-700">{item.tokens}</td>
                  <td className="py-3.5 text-gray-700 font-semibold">{item.revShare}</td>
                  <td className="py-3.5 font-bold text-emerald-700 text-right">{item.accrual}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>
    </div>
  );
}
