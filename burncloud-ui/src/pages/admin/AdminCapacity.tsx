import React, { useState } from 'react';
import {
  Zap
} from 'lucide-react';
import {
  BCPageHeader,
  BCMetric,
  BCCard,
  BCButton,
  BCStatus
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminCapacity() {
  const { t } = useTranslation();
  const [autoScaleEnabled, setAutoScaleEnabled] = useState(true);

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.capacity.title}
        subtitle={t.admin.capacity.subtitle}
        conclusion={{
          text: t.admin.capacity.conclusion,
          type: 'healthy'
        }}
        actions={
          <div className="flex items-center gap-2">
            <BCButton
              variant={autoScaleEnabled ? 'primary' : 'secondary'}
              size="sm"
              onClick={() => setAutoScaleEnabled(!autoScaleEnabled)}
            >
              <Zap className="w-3.5 h-3.5" />
              <span>{t.admin.capacity.btnAutopilotScaling}: {autoScaleEnabled ? 'ACTIVE' : 'MANUAL'}</span>
            </BCButton>
          </div>
        }
      />

      {/* Primary Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <BCMetric
          label={t.admin.capacity.metricGlobalHeadroom}
          value="34.2%"
          trend="Healthy safety margin"
          trendPositive={true}
        />
        <BCMetric
          label={t.admin.capacity.metricPeakConcurrency}
          value="48,200"
          unit="tokens/s"
          subtitle="US-West (SJC) peak"
        />
        <BCMetric
          label={t.admin.capacity.metricProvisionedReplicas}
          value="58"
          unit="replicas"
          subtitle="Across 6 frontier models"
        />
        <BCMetric
          label={t.admin.capacity.metricTargetLatency}
          value="< 200ms"
          status={<BCStatus status="Healthy" />}
        />
      </div>

      {/* Model Capacity Headroom Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.capacity.headroomTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.capacity.headroomSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.capacity.colModel}</th>
                <th className="pb-3 font-semibold">{t.admin.capacity.colProvisionedCapacity}</th>
                <th className="pb-3 font-semibold">{t.admin.capacity.colActiveDemand}</th>
                <th className="pb-3 font-semibold">{t.admin.capacity.colHeadroom}</th>
                <th className="pb-3 font-semibold">{t.admin.capacity.colP95Latency}</th>
                <th className="pb-3 font-semibold">{t.admin.capacity.colScalingAction}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.capacity.colStatus}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  model: 'DeepSeek V3 (671B MoE)',
                  provisioned: '32,000 tok/s',
                  demand: '24,800 tok/s',
                  headroom: '22.5%',
                  p95: '148 ms',
                  action: 'Auto-scaled +2 nodes',
                  status: 'Healthy'
                },
                {
                  model: 'DeepSeek R1 Reasoning',
                  provisioned: '18,000 tok/s',
                  demand: '12,400 tok/s',
                  headroom: '31.1%',
                  p95: '380 ms',
                  action: 'Stable',
                  status: 'Healthy'
                },
                {
                  model: 'Qwen 2.5 72B Instruct',
                  provisioned: '14,000 tok/s',
                  demand: '8,900 tok/s',
                  headroom: '36.4%',
                  p95: '190 ms',
                  action: 'Stable',
                  status: 'Healthy'
                },
                {
                  model: 'GLM-4 Plus',
                  provisioned: '8,000 tok/s',
                  demand: '6,800 tok/s',
                  headroom: '15.0%',
                  p95: '780 ms',
                  action: 'Standby node provisioning',
                  status: 'Degraded'
                }
              ].map((item, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-bold font-sans text-gray-900">{item.model}</td>
                  <td className="py-3.5 text-gray-700">{item.provisioned}</td>
                  <td className="py-3.5 font-bold text-gray-950">{item.demand}</td>
                  <td className="py-3.5">
                    <span className={parseFloat(item.headroom) < 20 ? 'text-amber-600 font-bold' : 'text-emerald-700 font-bold'}>
                      {item.headroom}
                    </span>
                  </td>
                  <td className="py-3.5 text-gray-700">{item.p95}</td>
                  <td className="py-3.5 font-sans text-gray-600">{item.action}</td>
                  <td className="py-3.5 text-right font-sans">
                    <BCStatus status={item.status} />
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
