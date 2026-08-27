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

export function AdminRevenue() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.revenue.title}
        subtitle={t.admin.revenue.subtitle}
        conclusion={{
          text: t.admin.revenue.conclusion,
          type: 'healthy'
        }}
        actions={
          <BCButton variant="secondary" size="sm">
            <Download className="w-3.5 h-3.5" />
            <span>{t.admin.revenue.exportLedger}</span>
          </BCButton>
        }
      />

      {/* Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <BCMetric
          label={t.admin.revenue.metricPlatformGmvMonth}
          value="$384,200"
          trend="+28.4% MoM"
          trendPositive={true}
        />
        <BCMetric
          label={t.admin.revenue.metricSupplierCogs}
          value="$259,720"
          subtitle="67.6% of GMV payouts"
        />
        <BCMetric
          label={t.admin.revenue.metricNetPlatformMargin}
          value="$124,480"
          trend="32.4% Net Margin"
          trendPositive={true}
        />
        <BCMetric
          label={t.admin.revenue.metricPrepaidFloat}
          value="$842,100"
          subtitle="Prepaid customer credits"
        />
      </div>

      {/* Profitability by Model */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.revenue.profitabilityTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.revenue.profitabilitySubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.revenue.colModelEndpoint}</th>
                <th className="pb-3 font-semibold">{t.admin.revenue.colVolumeTokens}</th>
                <th className="pb-3 font-semibold">{t.admin.revenue.colGrossGmv}</th>
                <th className="pb-3 font-semibold">{t.admin.revenue.colSupplierCogs}</th>
                <th className="pb-3 font-semibold">{t.admin.revenue.colNetProfit}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.revenue.colMarginPercent}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  model: 'DeepSeek V3 (671B)',
                  volume: '1,420.0M',
                  gmv: '$148,200',
                  cogs: '$98,400',
                  profit: '$49,800',
                  margin: '33.6%'
                },
                {
                  model: 'DeepSeek R1 Reasoning',
                  volume: '680.0M',
                  gmv: '$112,400',
                  cogs: '$74,200',
                  profit: '$38,200',
                  margin: '34.0%'
                },
                {
                  model: 'Qwen 2.5 72B',
                  volume: '420.0M',
                  gmv: '$64,100',
                  cogs: '$44,800',
                  profit: '$19,300',
                  margin: '30.1%'
                },
                {
                  model: 'Claude 3.5 Sonnet Pass-Through',
                  volume: '140.0M',
                  gmv: '$59,500',
                  cogs: '$42,320',
                  profit: '$17,180',
                  margin: '28.9%'
                }
              ].map((item, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-bold font-sans text-gray-900">{item.model}</td>
                  <td className="py-3.5 text-gray-700">{item.volume}</td>
                  <td className="py-3.5 font-bold text-gray-950">{item.gmv}</td>
                  <td className="py-3.5 text-gray-600">{item.cogs}</td>
                  <td className="py-3.5 font-bold text-emerald-700">{item.profit}</td>
                  <td className="py-3.5 font-bold text-emerald-700 text-right">{item.margin}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>
    </div>
  );
}
