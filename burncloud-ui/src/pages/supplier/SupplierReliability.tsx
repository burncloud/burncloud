import React from 'react';
import {
  BCPageHeader,
  BCMetric,
  BCCard,
  BCBadge
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function SupplierReliability() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.supplier.reliability.title}
        subtitle={t.supplier.reliability.subtitle}
        conclusion={{
          text: t.supplier.reliability.conclusion,
          type: 'healthy'
        }}
      />

      {/* Primary Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <BCMetric
          label={t.supplier.reliability.metricScore}
          value="98.6"
          unit="/ 100"
          status={<BCBadge variant="success" size="sm">EXCELLENT</BCBadge>}
          subtitle="Top 5% of network suppliers"
        />
        <BCMetric
          label={t.supplier.reliability.metricUptime}
          value="99.94%"
          trend="0 unannounced drops"
          trendPositive={true}
        />
        <BCMetric
          label={t.supplier.reliability.metricVerification}
          value="L3 Pro"
          subtitle="Enterprise SLA qualified"
        />
        <BCMetric
          label={t.supplier.reliability.metricRevShareMultiplier}
          value="80.0%"
          subtitle="Base is 70%"
        />
      </div>

      {/* Tier Roadmap Card */}
      <BCCard className="p-6 space-y-4">
        <h3 className="text-base font-bold text-gray-950">{t.supplier.reliability.progressionTitle}</h3>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4 pt-2">
          {[
            {
              tier: 'L1 Community',
              share: '70% Rev Share',
              req: '1+ GPU Node • Consumer or Data Center',
              status: 'Completed'
            },
            {
              tier: 'L2 Verified',
              share: '75% Rev Share',
              req: '8+ Enterprise GPUs • 99.5% Uptime',
              status: 'Completed'
            },
            {
              tier: 'L3 Professional',
              share: '80% Rev Share',
              req: '24+ SXM GPUs • NVLink • Sub-20ms Jitter',
              status: 'Current Tier'
            },
            {
              tier: 'L4 Strategic',
              share: '85% Rev Share',
              req: '64+ H100/B200 Clusters • SOC2 Type II',
              status: 'Eligible with +36 GPUs'
            }
          ].map((item, idx) => (
            <div
              key={idx}
              className={`p-4 rounded-xl border space-y-2 ${
                item.status === 'Current Tier'
                  ? 'bg-emerald-50/70 border-emerald-300 ring-2 ring-emerald-500/20'
                  : item.status === 'Completed'
                  ? 'bg-gray-50 border-gray-200 opacity-90'
                  : 'bg-white border-gray-200'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-bold text-xs text-gray-900">{item.tier}</span>
                <BCBadge
                  variant={
                    item.status === 'Current Tier'
                      ? 'success'
                      : item.status === 'Completed'
                      ? 'neutral'
                      : 'brand'
                  }
                  size="sm"
                >
                  {item.status}
                </BCBadge>
              </div>
              <div className="text-sm font-bold font-mono text-gray-950">{item.share}</div>
              <p className="text-[11px] text-gray-500">{item.req}</p>
            </div>
          ))}
        </div>
      </BCCard>

      {/* Incident & Audit History */}
      <BCCard className="p-6 space-y-4">
        <h3 className="text-base font-bold text-gray-950">{t.supplier.reliability.eventLogTitle}</h3>
        <div className="space-y-3">
          {[
            {
              time: 'Today 11:20 AM',
              title: 'Thermal Warning Handled by Autopilot on HKG-Edge-RTX-Pool',
              desc: 'Core temperature reached 72°C. Autopilot rerouted 100% of live traffic without customer disruption.',
              status: 'Mitigated'
            },
            {
              time: 'Aug 14, 2026',
              title: 'Routine Driver Update on SJC-Pod-01-Rack4',
              desc: 'Graceful drain executed for 18 minutes. Zero SLA penalties applied.',
              status: 'Planned'
            },
            {
              time: 'Aug 02, 2026',
              title: 'NVLink Interconnect Benchmark Passed',
              desc: '8x H100 SXM5 verified at 894 GB/s bi-directional throughput.',
              status: 'Verified'
            }
          ].map((item, idx) => (
            <div key={idx} className="p-3.5 bg-gray-50 rounded-xl border border-gray-100 flex items-start justify-between gap-4 text-xs">
              <div>
                <div className="font-bold text-gray-900">{item.title}</div>
                <div className="text-gray-500 text-[11px] mt-0.5">{item.desc}</div>
              </div>
              <BCBadge variant="neutral" size="sm" className="flex-shrink-0">
                {item.status}
              </BCBadge>
            </div>
          ))}
        </div>
      </BCCard>
    </div>
  );
}
