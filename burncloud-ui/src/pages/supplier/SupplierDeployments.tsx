import React from 'react';
import {
  ShieldCheck
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCStatus
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function SupplierDeployments() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.supplier.deployments.title}
        subtitle={t.supplier.deployments.subtitle}
        conclusion={{
          text: t.supplier.deployments.conclusion,
          type: 'healthy'
        }}
      />

      {/* Deployments Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.supplier.deployments.title}</h3>
            <p className="text-xs text-gray-500">{t.supplier.deployments.subtitle}</p>
          </div>
          <div className="flex items-center gap-2 text-xs font-mono text-emerald-700 bg-emerald-50 px-2.5 py-1 rounded-lg border border-emerald-200">
            <ShieldCheck className="w-4 h-4" />
            <span>{t.supplier.deployments.colAutopilotStatus}</span>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.supplier.deployments.colModel}</th>
                <th className="pb-3 font-semibold">{t.supplier.deployments.colNodes}</th>
                <th className="pb-3 font-semibold">{t.supplier.deployments.colTensorParallel}</th>
                <th className="pb-3 font-semibold">{t.supplier.deployments.colThroughput}</th>
                <th className="pb-3 font-semibold">{t.supplier.deployments.colContribution}</th>
                <th className="pb-3 font-semibold text-right">{t.supplier.deployments.colAutopilotStatus}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 font-mono">
              {[
                {
                  model: 'DeepSeek V3 (671B MoE)',
                  nodes: 'SJC-Pod-01-Rack4 (8x H100)',
                  tp: 'TP=8 / FP8 FlashInfer',
                  throughput: '85.2 tokens/s',
                  score: '99.8 / 100',
                  status: 'Healthy'
                },
                {
                  model: 'DeepSeek R1 Reasoning',
                  nodes: 'SJC-Pod-01-Rack5 (8x H100)',
                  tp: 'TP=8 / DeepSeek-VLLM',
                  throughput: '56.4 tokens/s',
                  score: '99.4 / 100',
                  status: 'Healthy'
                },
                {
                  model: 'Qwen 2.5 72B Instruct',
                  nodes: 'FRA-DC2-Compute-08 (8x A100)',
                  tp: 'TP=8 / vLLM v0.6.2',
                  throughput: '72.0 tokens/s',
                  score: '98.9 / 100',
                  status: 'Healthy'
                },
                {
                  model: 'Llama 3.3 70B Quantized',
                  nodes: 'HKG-Edge-RTX-Pool (4x 4090)',
                  tp: 'TP=4 / AWQ INT4',
                  throughput: '42.1 tokens/s',
                  score: '92.1 / 100',
                  status: 'Degraded'
                }
              ].map((dep, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-sans font-bold text-gray-900">{dep.model}</td>
                  <td className="py-3.5 text-gray-700 font-sans">{dep.nodes}</td>
                  <td className="py-3.5 text-gray-600">{dep.tp}</td>
                  <td className="py-3.5 text-gray-900 font-bold">{dep.throughput}</td>
                  <td className="py-3.5 text-emerald-700 font-bold">{dep.score}</td>
                  <td className="py-3.5 text-right font-sans">
                    <BCStatus status={dep.status} />
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
