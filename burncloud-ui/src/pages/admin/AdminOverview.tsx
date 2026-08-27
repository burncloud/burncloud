import React from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Server,
  TrendingUp,
  AlertTriangle,
  ArrowRight,
  Gauge,
  Workflow
} from 'lucide-react';
import {
  BCPageHeader,
  BCMetric,
  BCCard,
  BCButton,
  BCBadge,
  BCStatus
} from '@/components/ui';
import { useRole } from '@/context/RoleContext';
import { MOCK_AUTOPILOT_LOGS } from '@/data/workbenchData';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminOverview() {
  const navigate = useNavigate();
  const { adminRevenueToday } = useRole();
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* 1. Page Header */}
      <BCPageHeader
        title={t.admin.overview.title}
        subtitle={t.admin.overview.subtitle}
        conclusion={{
          text: t.admin.overview.conclusion,
          type: 'healthy'
        }}
        actions={
          <div className="flex items-center gap-2">
            <BCButton
              variant="secondary"
              size="sm"
              onClick={() => navigate('/admin/operations')}
            >
              <Workflow className="w-3.5 h-3.5" />
              <span>{t.admin.overview.actionAutopilotCenter}</span>
            </BCButton>
            <BCButton
              variant="primary"
              size="sm"
              onClick={() => navigate('/admin/capacity')}
            >
              <Gauge className="w-3.5 h-3.5" />
              <span>{t.admin.overview.actionManageHeadroom}</span>
            </BCButton>
          </div>
        }
      />

      {/* 2. Four Primary Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* 1: Today Revenue */}
        <BCMetric
          label={t.admin.overview.metricPlatformGmv}
          value={`$${adminRevenueToday.toLocaleString()}`}
          trend="+22.8% vs last week"
          trendPositive={true}
          subtitle={t.admin.overview.metricPlatformGmvSub}
        />

        {/* 2: Gross Margin */}
        <BCMetric
          label={t.admin.overview.metricGrossMargin}
          value="32.4%"
          trend="+1.8% vs target"
          trendPositive={true}
          subtitle={t.admin.overview.metricGrossMarginSub}
        />

        {/* 3: Online GPU Capacity */}
        <BCMetric
          label={t.admin.overview.metricOnlineGpus}
          value="420"
          unit="GPUs"
          status={<BCStatus status="Online" label="1,840 TFLOPS" />}
          subtitle={t.admin.overview.metricOnlineGpusSub}
        />

        {/* 4: API Availability */}
        <BCMetric
          label={t.admin.overview.metricGlobalSla}
          value="99.99%"
          status={<BCStatus status="Healthy" />}
          subtitle={t.admin.overview.metricGlobalSlaSub}
        />
      </div>

      {/* 3. Needs Attention / Capacity Risk Banner */}
      <div id="attention" className="p-4 rounded-2xl bg-amber-50/80 border border-amber-200 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div className="flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-amber-600 flex-shrink-0 mt-0.5" />
          <div>
            <h4 className="text-sm font-bold text-amber-950">{t.admin.overview.capacityWarningTitle}</h4>
            <p className="text-xs text-amber-800 mt-0.5">
              {t.admin.overview.capacityWarningDesc}
            </p>
          </div>
        </div>
        <BCButton
          variant="primary"
          size="sm"
          onClick={() => navigate('/admin/capacity')}
          className="flex-shrink-0 bg-amber-900 hover:bg-amber-800 border-amber-900"
        >
          <span>{t.admin.overview.actionAdjustPolicy}</span>
        </BCButton>
      </div>

      {/* 4. Real-time Autopilot Autonomous Actions Feed */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.overview.autopilotActionsTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.overview.autopilotActionsSubtitle}</p>
          </div>
          <BCButton
            variant="ghost"
            size="sm"
            onClick={() => navigate('/admin/operations')}
            className="text-xs"
          >
            <span>{t.admin.overview.fullIncidentLog}</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </BCButton>
        </div>

        <div className="space-y-3">
          {MOCK_AUTOPILOT_LOGS.map((item) => (
            <div key={item.id} className="p-4 rounded-xl bg-gray-50 border border-gray-100 flex flex-col sm:flex-row sm:items-start justify-between gap-3 text-xs">
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <BCBadge
                    variant={item.level === 'Action' ? 'brand' : item.level === 'Optimization' ? 'success' : 'warning'}
                    size="sm"
                  >
                    {item.category}
                  </BCBadge>
                  <span className="font-bold text-gray-900">{item.title}</span>
                </div>
                <p className="text-gray-600 text-[11px] leading-relaxed">{item.description}</p>
                <div className="flex items-center gap-3 pt-1 text-[11px] font-mono">
                  <span className="text-emerald-700 font-semibold">Impact: {item.impact}</span>
                  <span className="text-gray-400">•</span>
                  <span className="text-gray-500">{item.actionTaken}</span>
                </div>
              </div>
              <span className="text-[10px] font-mono text-gray-400 whitespace-nowrap">{item.time}</span>
            </div>
          ))}
        </div>
      </BCCard>

      {/* 5. Two Column: Supply Status & Demand Distribution */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
        {/* Supply Summary */}
        <BCCard className="p-6 space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold text-gray-950 flex items-center gap-2">
              <Server className="w-4 h-4 text-gray-700" />
              <span>{t.admin.overview.supplySourcesTitle}</span>
            </h3>
            <button
              onClick={() => navigate('/admin/supply')}
              className="text-xs font-semibold text-blue-600 hover:underline cursor-pointer"
            >
              {t.admin.overview.viewSupplyFleets} →
            </button>
          </div>

          <div className="space-y-2 text-xs font-mono pt-1">
            <div className="flex justify-between p-2 bg-gray-50 rounded-lg">
              <span className="text-gray-600 font-sans">{t.admin.overview.verifiedSuppliers}</span>
              <span className="font-bold text-gray-900">280 GPUs (66.7%)</span>
            </div>
            <div className="flex justify-between p-2 bg-gray-50 rounded-lg">
              <span className="text-gray-600 font-sans">{t.admin.overview.ownedBareMetal}</span>
              <span className="font-bold text-gray-900">96 GPUs (22.8%)</span>
            </div>
            <div className="flex justify-between p-2 bg-gray-50 rounded-lg">
              <span className="text-gray-600 font-sans">{t.admin.overview.elasticFallbacks}</span>
              <span className="font-bold text-gray-900">44 GPUs (10.5%)</span>
            </div>
          </div>
        </BCCard>

        {/* Demand Pressure Summary */}
        <BCCard className="p-6 space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold text-gray-950 flex items-center gap-2">
              <TrendingUp className="w-4 h-4 text-gray-700" />
              <span>{t.admin.overview.demandPressureTitle}</span>
            </h3>
            <button
              onClick={() => navigate('/admin/demand')}
              className="text-xs font-semibold text-blue-600 hover:underline cursor-pointer"
            >
              {t.admin.overview.viewVelocity} →
            </button>
          </div>

          <div className="space-y-2 text-xs font-mono pt-1">
            <div className="flex justify-between p-2 bg-gray-50 rounded-lg">
              <span className="text-gray-600 font-sans">{t.admin.overview.currentVelocity}</span>
              <span className="font-bold text-gray-900">42,800 tokens / sec</span>
            </div>
            <div className="flex justify-between p-2 bg-gray-50 rounded-lg">
              <span className="text-gray-600 font-sans">{t.admin.overview.peakConcurrency}</span>
              <span className="font-bold text-amber-700">89.2% (High)</span>
            </div>
            <div className="flex justify-between p-2 bg-gray-50 rounded-lg">
              <span className="text-gray-600 font-sans">{t.admin.overview.activeTenants}</span>
              <span className="font-bold text-gray-900">1,420 Active Teams</span>
            </div>
          </div>
        </BCCard>
      </div>
    </div>
  );
}
