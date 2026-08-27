import React from 'react';
import { useNavigate } from 'react-router-dom';
import {
  DollarSign,
  CreditCard,
  Zap,
  ArrowRight,
  Key,
  Terminal,
  Store,
  AlertTriangle
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
import { WORKBENCH_MODELS } from '@/data/workbenchData';
import { useTranslation } from '@/i18n/I18nContext';

export function BuyerOverview() {
  const navigate = useNavigate();
  const { balance, todaySpend } = useRole();
  const { t } = useTranslation();

  const isLowBalance = balance < 20;

  return (
    <div className="space-y-6">
      {/* 1. Page Header & Intelligent Conclusion */}
      <BCPageHeader
        title={t.buyer.overview.title}
        subtitle={t.buyer.overview.subtitle}
        conclusion={{
          text: isLowBalance
            ? t.buyer.overview.conclusionWarning
            : t.buyer.overview.conclusionHealthy,
          type: isLowBalance ? 'warning' : 'healthy'
        }}
        actions={
          <div className="flex items-center gap-2">
            <BCButton
              variant="secondary"
              size="sm"
              onClick={() => navigate('/buyer/playground')}
            >
              <Terminal className="w-3.5 h-3.5" />
              <span>{t.buyer.overview.openPlayground}</span>
            </BCButton>
            <BCButton
              variant="primary"
              size="sm"
              onClick={() => navigate('/buyer/marketplace')}
            >
              <Store className="w-3.5 h-3.5" />
              <span>{t.buyer.overview.browseMarketplace}</span>
            </BCButton>
          </div>
        }
      />

      {/* 2. Four Primary Metrics (Fixed Order per Contract) */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* 1: Today Spend */}
        <BCMetric
          label={t.buyer.overview.metricTodaySpend}
          value={`$${todaySpend.toFixed(2)}`}
          trend="+8.4% vs yesterday"
          trendPositive={false}
          subtitle={t.buyer.overview.metricTodaySpendSub}
        />

        {/* 2: Balance */}
        <BCMetric
          label={t.buyer.overview.metricBalance}
          value={`$${balance.toFixed(2)}`}
          status={
            isLowBalance ? (
              <BCBadge variant="warning" size="sm">LOW</BCBadge>
            ) : (
              <BCBadge variant="success" size="sm">HEALTHY</BCBadge>
            )
          }
          subtitle={t.buyer.overview.metricBalanceSub}
        />

        {/* 3: API Availability */}
        <BCMetric
          label={t.buyer.overview.metricApiAvailability}
          value="99.99%"
          status={<BCStatus status="Healthy" />}
          subtitle={t.buyer.overview.metricApiAvailabilitySub}
        />

        {/* 4: Tokens Today */}
        <BCMetric
          label={t.buyer.overview.metricTokensToday}
          value="1.84M"
          unit="tokens"
          trend="85.4 req/min peak"
          trendPositive={true}
          subtitle={t.buyer.overview.metricTokensTodaySub}
        />
      </div>

      {/* 3. Needs Attention (Only shown when action is required) */}
      {isLowBalance && (
        <div id="attention" className="p-4 rounded-2xl bg-amber-50/80 border border-amber-200 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="flex items-start gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-600 flex-shrink-0 mt-0.5" />
            <div>
              <h4 className="text-sm font-bold text-amber-950">{t.buyer.overview.attentionTitle}</h4>
              <p className="text-xs text-amber-800 mt-0.5">
                {t.buyer.overview.attentionDesc}
              </p>
            </div>
          </div>
          <BCButton
            variant="primary"
            size="sm"
            onClick={() => navigate('/buyer/billing')}
            className="flex-shrink-0 bg-amber-900 hover:bg-amber-800 border-amber-900"
          >
            <CreditCard className="w-3.5 h-3.5" />
            <span>{t.buyer.overview.attentionTopUpBtn}</span>
          </BCButton>
        </div>
      )}

      {/* 4. Main Section: Models in Use */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.buyer.overview.modelsInUseTitle}</h3>
            <p className="text-xs text-gray-500">{t.buyer.overview.modelsInUseDesc}</p>
          </div>
          <BCButton
            variant="ghost"
            size="sm"
            onClick={() => navigate('/buyer/marketplace')}
            className="text-xs"
          >
            <span>{t.buyer.overview.exploreAllModels}</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </BCButton>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 font-mono uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.buyer.overview.colModel}</th>
                <th className="pb-3 font-semibold">{t.buyer.overview.colSelectedTier}</th>
                <th className="pb-3 font-semibold">{t.buyer.overview.colTodayTokens}</th>
                <th className="pb-3 font-semibold">{t.buyer.overview.colP95Latency}</th>
                <th className="pb-3 font-semibold">{t.buyer.overview.colTodayCost}</th>
                <th className="pb-3 font-semibold">{t.buyer.overview.colServiceStatus}</th>
                <th className="pb-3 font-semibold text-right">{t.buyer.overview.colAction}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  model: WORKBENCH_MODELS[0], // DeepSeek V3
                  tier: 'Standard',
                  tokens: '1,120,400',
                  cost: '$0.28',
                  p95: '380 ms',
                  status: 'Healthy'
                },
                {
                  model: WORKBENCH_MODELS[1], // DeepSeek R1
                  tier: 'Performance',
                  tokens: '410,200',
                  cost: '$0.89',
                  p95: '620 ms',
                  status: 'Healthy'
                },
                {
                  model: WORKBENCH_MODELS[2], // Qwen 2.5 72B
                  tier: 'Standard',
                  tokens: '310,000',
                  cost: '$0.18',
                  p95: '410 ms',
                  status: 'Healthy'
                }
              ].map((item, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70 transition-colors">
                  <td className="py-3.5 font-medium text-gray-950">
                    <div className="flex items-center gap-2">
                      <div className="w-6 h-6 rounded-lg bg-gray-100 flex items-center justify-center font-mono font-bold text-[10px] text-gray-700">
                        {item.model.name[0]}
                      </div>
                      <div>
                        <div className="font-semibold text-gray-900">{item.model.name}</div>
                        <div className="text-[10px] font-mono text-gray-500">{item.model.family}</div>
                      </div>
                    </div>
                  </td>
                  <td className="py-3.5">
                    <BCBadge variant={item.tier === 'Performance' ? 'accent' : 'neutral'} size="sm">
                      {item.tier}
                    </BCBadge>
                  </td>
                  <td className="py-3.5 font-mono text-gray-700 font-medium">{item.tokens}</td>
                  <td className="py-3.5 font-mono text-gray-700">{item.p95}</td>
                  <td className="py-3.5 font-mono font-semibold text-gray-950">{item.cost}</td>
                  <td className="py-3.5">
                    <BCStatus status={item.status} />
                  </td>
                  <td className="py-3.5 text-right">
                    <button
                      onClick={() => navigate('/buyer/playground')}
                      className="text-blue-600 hover:text-blue-800 font-semibold font-mono text-xs hover:underline cursor-pointer"
                    >
                      {t.buyer.overview.testInPlayground}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>

      {/* 5. Recent Activity Stream */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.buyer.overview.recentActivityTitle}</h3>
            <p className="text-xs text-gray-500">{t.buyer.overview.recentActivityDesc}</p>
          </div>
          <BCButton
            variant="ghost"
            size="sm"
            onClick={() => navigate('/buyer/logs')}
            className="text-xs"
          >
            <span>{t.buyer.overview.viewFullLogs}</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </BCButton>
        </div>

        <div className="space-y-3">
          {[
            {
              time: '12 mins ago',
              type: 'Recharge',
              title: 'Prepaid balance top-up completed ($100.00)',
              desc: 'Receipt #REC-8921 generated. Payment method: Visa ending in 4242.'
            },
            {
              time: '1 hour ago',
              type: 'Autopilot',
              title: 'Sub-150ms smart fallback verified for DeepSeek V3',
              desc: 'BurnCloud automatically provisioned additional capacity in US-West cluster.'
            },
            {
              time: '1 day ago',
              type: 'Key',
              title: 'New API Key "Production Kubernetes Cluster" generated',
              desc: 'Associated with Standard & Performance tiers.'
            }
          ].map((event, idx) => (
            <div key={idx} className="p-3.5 rounded-xl bg-gray-50/80 border border-gray-100 flex items-start justify-between gap-4">
              <div className="flex items-start gap-3">
                <div className="w-7 h-7 rounded-lg bg-white border border-gray-200 flex items-center justify-center text-gray-700 flex-shrink-0 mt-0.5 shadow-xs">
                  {event.type === 'Recharge' ? <DollarSign className="w-3.5 h-3.5 text-emerald-600" /> : event.type === 'Autopilot' ? <Zap className="w-3.5 h-3.5 text-blue-600" /> : <Key className="w-3.5 h-3.5 text-gray-600" />}
                </div>
                <div>
                  <div className="font-semibold text-gray-900 text-xs">{event.title}</div>
                  <div className="text-[11px] text-gray-500 mt-0.5">{event.desc}</div>
                </div>
              </div>
              <span className="text-[10px] font-mono text-gray-400 whitespace-nowrap">{event.time}</span>
            </div>
          ))}
        </div>
      </BCCard>
    </div>
  );
}
