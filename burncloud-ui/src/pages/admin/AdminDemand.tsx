import React from 'react';
import {
  BCPageHeader,
  BCMetric,
  BCCard,
  BCBadge
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminDemand() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.demand.title}
        subtitle={t.admin.demand.subtitle}
        conclusion={{
          text: t.admin.demand.conclusion,
          type: 'healthy'
        }}
      />

      {/* Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <BCMetric
          label={t.admin.demand.metricLiveVelocity}
          value="42,800"
          unit="tokens/s"
          trend="+14.8% vs 1h ago"
          trendPositive={true}
        />
        <BCMetric
          label={t.admin.demand.metricActiveTenants}
          value="1,420"
          subtitle="Top 50 account for 68% volume"
        />
        <BCMetric
          label={t.admin.demand.metricRegionalShare}
          value="58.2%"
          subtitle="Silicon Valley low-latency cluster"
        />
        <BCMetric
          label={t.admin.demand.metricAvgRequestSize}
          value="1,420"
          unit="tokens"
          subtitle="Prompt + Completion"
        />
      </div>

      {/* Top Consuming Tenants */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.demand.topTenantsTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.demand.topTenantsSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.demand.colTenant}</th>
                <th className="pb-3 font-semibold">{t.admin.demand.colTierPref}</th>
                <th className="pb-3 font-semibold">{t.admin.demand.colTokens24h}</th>
                <th className="pb-3 font-semibold">{t.admin.demand.colSpend24h}</th>
                <th className="pb-3 font-semibold">{t.admin.demand.colPrimaryModel}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.demand.colCreditBalance}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  name: 'HyperScale AI Labs (US-West)',
                  tier: 'Performance Tier',
                  tokens: '148.2M',
                  spend: '$4,280.00',
                  model: 'DeepSeek V3 (671B)',
                  balance: '$18,400.00'
                },
                {
                  name: 'Apex Robotics Co.',
                  tier: 'Standard Tier',
                  tokens: '84.1M',
                  spend: '$2,140.50',
                  model: 'DeepSeek R1',
                  balance: '$6,200.00'
                },
                {
                  name: 'FinTech Intelligence Inc.',
                  tier: 'Performance Tier',
                  tokens: '52.0M',
                  spend: '$1,920.00',
                  model: 'Qwen 2.5 72B',
                  balance: '$12,850.00'
                }
              ].map((tenant, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-sans font-bold text-gray-900">{tenant.name}</td>
                  <td className="py-3.5">
                    <BCBadge variant={tenant.tier.includes('Performance') ? 'accent' : 'neutral'} size="sm">
                      {tenant.tier}
                    </BCBadge>
                  </td>
                  <td className="py-3.5 text-gray-700 font-bold">{tenant.tokens}</td>
                  <td className="py-3.5 text-gray-950 font-bold">{tenant.spend}</td>
                  <td className="py-3.5 font-sans text-gray-700">{tenant.model}</td>
                  <td className="py-3.5 text-emerald-700 font-bold text-right">{tenant.balance}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>
    </div>
  );
}
