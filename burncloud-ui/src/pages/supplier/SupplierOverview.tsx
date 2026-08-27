import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Server,
  AlertTriangle,
  ArrowRight,
  Plus
} from 'lucide-react';
import {
  BCPageHeader,
  BCMetric,
  BCCard,
  BCButton,
  BCStatus,
  BCModal
} from '@/components/ui';
import { useRole } from '@/context/RoleContext';
import { MOCK_SUPPLIER_NODES } from '@/data/workbenchData';
import { useTranslation } from '@/i18n/I18nContext';

export function SupplierOverview() {
  const navigate = useNavigate();
  const { supplierEarningsToday } = useRole();
  const { t } = useTranslation();
  const [isOnboardModalOpen, setIsOnboardModalOpen] = useState(false);

  const totalGpus = MOCK_SUPPLIER_NODES.reduce((acc, n) => acc + n.gpuCount, 0);
  const onlineGpus = MOCK_SUPPLIER_NODES.filter(n => n.status === 'Online').reduce((acc, n) => acc + n.gpuCount, 0);
  const avgUtilization = (
    MOCK_SUPPLIER_NODES.reduce((acc, n) => acc + n.utilization, 0) / MOCK_SUPPLIER_NODES.length
  ).toFixed(1);

  const degradedNode = MOCK_SUPPLIER_NODES.find(n => n.status === 'Degraded');

  return (
    <div className="space-y-6">
      {/* 1. Page Header */}
      <BCPageHeader
        title={t.supplier.overview.title}
        subtitle={t.supplier.overview.subtitle}
        conclusion={{
          text: degradedNode
            ? `${t.supplier.overview.attentionTitle}: ${degradedNode.name} (72°C).`
            : t.supplier.overview.conclusion,
          type: degradedNode ? 'warning' : 'healthy'
        }}
        actions={
          <div className="flex items-center gap-2">
            <BCButton
              variant="secondary"
              size="sm"
              onClick={() => navigate('/supplier/resources')}
            >
              <Server className="w-3.5 h-3.5" />
              <span>{t.supplier.overview.inspectNodes}</span>
            </BCButton>
            <BCButton
              variant="primary"
              size="sm"
              onClick={() => setIsOnboardModalOpen(true)}
            >
              <Plus className="w-3.5 h-3.5" />
              <span>{t.supplier.overview.connectNodeBtn}</span>
            </BCButton>
          </div>
        }
      />

      {/* 2. Four Primary Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* 1: Today Earnings */}
        <BCMetric
          label={t.supplier.overview.todayEarnings}
          value={`$${supplierEarningsToday.toFixed(2)}`}
          trend="+12.4% vs yesterday"
          trendPositive={true}
          subtitle={t.supplier.overview.todayEarningsSub}
        />

        {/* 2: Online GPUs */}
        <BCMetric
          label={t.supplier.overview.onlineGpus}
          value={`${onlineGpus} / ${totalGpus}`}
          unit="GPUs"
          status={<BCStatus status="Online" label="3 Clusters" />}
          subtitle={t.supplier.overview.onlineGpusSub}
        />

        {/* 3: GPU Utilization */}
        <BCMetric
          label={t.supplier.overview.gpuUtilization}
          value={`${avgUtilization}%`}
          trend="88.6% peak compute"
          trendPositive={true}
          subtitle={t.supplier.overview.gpuUtilizationSub}
        />

        {/* 4: Inference Today */}
        <BCMetric
          label={t.supplier.overview.inferenceTokens}
          value="24.2M"
          unit="tokens"
          subtitle={t.supplier.overview.inferenceTokensSub}
        />
      </div>

      {/* 3. Needs Attention Banner */}
      {degradedNode && (
        <div id="attention" className="p-4 rounded-2xl bg-amber-50/80 border border-amber-200 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="flex items-start gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-600 flex-shrink-0 mt-0.5" />
            <div>
              <h4 className="text-sm font-bold text-amber-950">
                {t.supplier.overview.attentionTitle} ({degradedNode.name})
              </h4>
              <p className="text-xs text-amber-800 mt-0.5">
                {t.supplier.overview.attentionDesc}
              </p>
            </div>
          </div>
          <BCButton
            variant="primary"
            size="sm"
            onClick={() => navigate('/supplier/resources')}
            className="flex-shrink-0 bg-amber-900 hover:bg-amber-800 border-amber-900"
          >
            <span>{t.supplier.overview.inspectNodes}</span>
          </BCButton>
        </div>
      )}

      {/* 4. Active Hardware Nodes Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.supplier.overview.connectedNodes}</h3>
            <p className="text-xs text-gray-500">{t.supplier.overview.connectedNodesDesc}</p>
          </div>
          <BCButton
            variant="ghost"
            size="sm"
            onClick={() => navigate('/supplier/resources')}
            className="text-xs"
          >
            <span>{t.supplier.overview.viewTopology}</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </BCButton>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 font-mono uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.supplier.overview.colNodeId}</th>
                <th className="pb-3 font-semibold">{t.supplier.overview.colGpuSpecs}</th>
                <th className="pb-3 font-semibold">{t.supplier.overview.colAssignedModel}</th>
                <th className="pb-3 font-semibold">{t.supplier.overview.colUtilization}</th>
                <th className="pb-3 font-semibold">{t.supplier.overview.colTemperature}</th>
                <th className="pb-3 font-semibold">{t.supplier.overview.colTodayEarnings}</th>
                <th className="pb-3 font-semibold">{t.supplier.overview.colStatus}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 font-mono">
              {MOCK_SUPPLIER_NODES.map((node) => (
                <tr key={node.id} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-sans">
                    <div className="font-bold text-gray-900">{node.name}</div>
                    <div className="text-[10px] font-mono text-gray-400">{node.region} • {node.cluster}</div>
                  </td>
                  <td className="py-3.5">
                    <div className="font-semibold text-gray-900 font-sans">{node.gpuCount}x {node.gpuType}</div>
                    <div className="text-[10px] text-gray-500">{node.vramTotalGb}GB VRAM • {node.pcieBandwidth}</div>
                  </td>
                  <td className="py-3.5 text-gray-700 font-sans">{node.assignedModel}</td>
                  <td className="py-3.5">
                    <span className="font-bold text-gray-900">{node.utilization}%</span>
                  </td>
                  <td className="py-3.5">
                    <span className={node.temperatureC > 70 ? 'text-amber-600 font-bold' : 'text-gray-600'}>
                      {node.temperatureC}°C
                    </span>
                  </td>
                  <td className="py-3.5 font-bold text-emerald-700">${node.earningsToday.toFixed(2)}</td>
                  <td className="py-3.5 font-sans">
                    <BCStatus status={node.status} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>

      {/* 5. Node Onboarding Modal */}
      <BCModal
        isOpen={isOnboardModalOpen}
        onClose={() => setIsOnboardModalOpen(false)}
        title={t.supplier.overview.modalTitle}
        subtitle={t.supplier.overview.modalSubtitle}
      >
        <div className="space-y-4 text-xs">
          <p className="text-gray-700 leading-relaxed font-sans">
            Install the lightweight node daemon on your Ubuntu / Debian GPU server (NVIDIA driver 535+ and CUDA 12.2+ required).
          </p>

          <div className="p-3 bg-gray-950 text-gray-100 rounded-xl font-mono text-[11px] space-y-2">
            <div className="text-gray-400 text-[10px]">{t.supplier.overview.commandComment}</div>
            <div className="text-emerald-400 select-all overflow-x-auto whitespace-pre">
              curl -sSL https://burncloud.io/install.sh | bash -s -- --token=demo-node-token
            </div>
          </div>

          <div className="p-3 bg-blue-50/80 rounded-xl border border-blue-200/80 space-y-1 font-sans text-blue-900">
            <div className="font-bold">{t.supplier.overview.verificationTitle}:</div>
            <p className="text-[11px] leading-relaxed">
              {t.supplier.overview.verificationDesc}
            </p>
          </div>

          <div className="pt-2 flex justify-end">
            <BCButton
              variant="primary"
              size="sm"
              onClick={() => setIsOnboardModalOpen(false)}
            >
              {t.supplier.overview.modalDone}
            </BCButton>
          </div>
        </div>
      </BCModal>
    </div>
  );
}
