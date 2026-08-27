import React from 'react';
import {
  BCPageHeader,
  BCMetric,
  BCCard,
  BCStatus
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminSupply() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.supply.title}
        subtitle={t.admin.supply.subtitle}
        conclusion={{
          text: t.admin.supply.conclusion,
          type: 'healthy'
        }}
      />

      {/* Metric Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <BCMetric
          label={t.admin.supply.metricTotalGpus}
          value="420"
          unit="GPUs"
          subtitle="96.2% fleet online rate"
        />
        <BCMetric
          label={t.admin.supply.metricVerifiedSuppliers}
          value="24"
          subtitle="Tier L2-L4 Providers"
        />
        <BCMetric
          label={t.admin.supply.metricBareMetalIdc}
          value="96"
          unit="GPUs"
          subtitle="US-West (SJC) Core DC"
        />
        <BCMetric
          label={t.admin.supply.metricBurstReserve}
          value="44"
          unit="GPUs"
          subtitle="Lambda Labs / RunPod standby"
        />
      </div>

      {/* Fleets Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.supply.clustersTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.supply.clustersSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.supply.colCluster}</th>
                <th className="pb-3 font-semibold">{t.admin.supply.colSourceType}</th>
                <th className="pb-3 font-semibold">{t.admin.supply.colGpuConfig}</th>
                <th className="pb-3 font-semibold">{t.admin.supply.colRegion}</th>
                <th className="pb-3 font-semibold">{t.admin.supply.colUtilization}</th>
                <th className="pb-3 font-semibold">{t.admin.supply.colAttestation}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.supply.colStatus}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  id: 'cls_sjc_core_01',
                  name: 'Silicon-Bay Core Pod 1-4',
                  source: 'Owned Bare-Metal IDC',
                  hw: '64x H100 SXM5 80GB',
                  region: 'us-west-sjc',
                  util: '92.4%',
                  attest: 'Confidential Nitro',
                  status: 'Online'
                },
                {
                  id: 'cls_sp_frankfurt_08',
                  name: 'Frankfurt EuroNode Alpha',
                  source: 'Supplier (L3 Verified)',
                  hw: '32x A100-SXM4 80GB',
                  region: 'eu-central-fra',
                  util: '78.1%',
                  attest: 'Hardware TPM',
                  status: 'Online'
                },
                {
                  id: 'cls_sp_tokyo_02',
                  name: 'Tokyo Enterprise Cluster',
                  source: 'Supplier (L4 Strategic)',
                  hw: '48x H100 SXM5 80GB',
                  region: 'ap-east-hkg',
                  util: '88.6%',
                  attest: 'Confidential Nitro',
                  status: 'Online'
                },
                {
                  id: 'cls_cloud_lambda_01',
                  name: 'Lambda Labs Elastic Burst',
                  source: 'External Cloud Reservation',
                  hw: '24x H100 PCIe',
                  region: 'us-east-va',
                  util: '32.0%',
                  attest: 'Cloud Attested',
                  status: 'Online'
                }
              ].map((c) => (
                <tr key={c.id} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-sans font-bold text-gray-900">{c.name}</td>
                  <td className="py-3.5 font-sans text-gray-700 font-medium">{c.source}</td>
                  <td className="py-3.5 text-gray-700">{c.hw}</td>
                  <td className="py-3.5 text-gray-500">{c.region}</td>
                  <td className="py-3.5 font-bold text-gray-900">{c.util}</td>
                  <td className="py-3.5 text-emerald-700 font-semibold">{c.attest}</td>
                  <td className="py-3.5 text-right font-sans">
                    <BCStatus status={c.status} />
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
