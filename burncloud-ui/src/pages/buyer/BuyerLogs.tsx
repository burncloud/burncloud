import React, { useState } from 'react';
import {
  ShieldCheck,
  RefreshCw
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCButton,
  BCBadge,
  BCDrawer,
  BCSearch
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

interface RequestLog {
  id: string;
  timestamp: string;
  model: string;
  tier: 'Economy' | 'Standard' | 'Performance';
  status: 200 | 429 | 503;
  ttftMs: number;
  totalLatencyMs: number;
  promptTokens: number;
  completionTokens: number;
  costUsd: number;
  apiKey: string;
  ipRegion: string;
  failoverOccurred?: boolean;
}

const MOCK_REQUEST_LOGS: RequestLog[] = [
  {
    id: 'req_98fa01b2',
    timestamp: '14:28:11.402',
    model: 'DeepSeek V3 (671B)',
    tier: 'Standard',
    status: 200,
    ttftMs: 138,
    totalLatencyMs: 442,
    promptTokens: 240,
    completionTokens: 820,
    costUsd: 0.000263,
    apiKey: 'demo-bc-prod••••',
    ipRegion: 'US-West (Oregon)'
  },
  {
    id: 'req_98fa01b1',
    timestamp: '14:27:54.110',
    model: 'DeepSeek R1 Reasoning',
    tier: 'Performance',
    status: 200,
    ttftMs: 290,
    totalLatencyMs: 1420,
    promptTokens: 110,
    completionTokens: 420,
    costUsd: 0.000980,
    apiKey: 'demo-bc-prod••••',
    ipRegion: 'US-West (Oregon)'
  },
  {
    id: 'req_98fa01b0',
    timestamp: '14:26:02.890',
    model: 'GLM-4 Plus',
    tier: 'Standard',
    status: 200,
    ttftMs: 410,
    totalLatencyMs: 820,
    promptTokens: 520,
    completionTokens: 310,
    costUsd: 0.000798,
    apiKey: 'demo-bc-agent••••',
    ipRegion: 'AP-East (Tokyo)',
    failoverOccurred: true
  },
  {
    id: 'req_98fa01af',
    timestamp: '14:24:19.012',
    model: 'Qwen 2.5 72B Instruct',
    tier: 'Standard',
    status: 200,
    ttftMs: 165,
    totalLatencyMs: 512,
    promptTokens: 80,
    completionTokens: 240,
    costUsd: 0.000200,
    apiKey: 'demo-bc-prod••••',
    ipRegion: 'EU-Central (Frankfurt)'
  }
];

export function BuyerLogs() {
  const { t } = useTranslation();
  const [logs, setLogs] = useState<RequestLog[]>(MOCK_REQUEST_LOGS);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedLog, setSelectedLog] = useState<RequestLog | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);

  const handleOpenLog = (log: RequestLog) => {
    setSelectedLog(log);
    setIsDrawerOpen(true);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.buyer.logs.title}
        subtitle={t.buyer.logs.subtitle}
        conclusion={{
          text: t.buyer.logs.conclusion,
          type: 'healthy'
        }}
      />

      {/* Filter and Search Bar */}
      <div className="flex items-center justify-between gap-3">
        <div className="w-72">
          <BCSearch
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.buyer.logs.searchPlaceholder}
          />
        </div>
        <BCButton
          variant="secondary"
          size="sm"
          onClick={() => {
            setLogs([...MOCK_REQUEST_LOGS]);
          }}
        >
          <RefreshCw className="w-3.5 h-3.5" />
          <span>{t.buyer.logs.filterAll}</span>
        </BCButton>
      </div>

      {/* Logs Table */}
      <BCCard className="p-6 space-y-4">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.buyer.logs.colTime}</th>
                <th className="pb-3 font-semibold">{t.buyer.logs.colRequestId}</th>
                <th className="pb-3 font-semibold">{t.buyer.logs.colModel}</th>
                <th className="pb-3 font-semibold">{t.buyer.logs.colStatus}</th>
                <th className="pb-3 font-semibold">{t.buyer.logs.colLatency}</th>
                <th className="pb-3 font-semibold">Total Latency</th>
                <th className="pb-3 font-semibold">{t.buyer.logs.colTokens}</th>
                <th className="pb-3 font-semibold">{t.buyer.logs.colCost}</th>
                <th className="pb-3 font-semibold text-right">{t.buyer.logs.colInspect}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {logs.map((log) => (
                <tr
                  key={log.id}
                  onClick={() => handleOpenLog(log)}
                  className="hover:bg-gray-50/80 cursor-pointer transition-colors"
                >
                  <td className="py-3 text-gray-500">{log.timestamp}</td>
                  <td className="py-3 font-bold text-gray-900">{log.id}</td>
                  <td className="py-3 font-sans">
                    <div className="font-semibold text-gray-900">{log.model}</div>
                    <div className="text-[10px] font-mono text-gray-400">{log.tier}</div>
                  </td>
                  <td className="py-3">
                    <BCBadge variant={log.status === 200 ? 'success' : 'error'} size="sm">
                      {log.status} OK
                    </BCBadge>
                  </td>
                  <td className="py-3 text-gray-700 font-semibold">{log.ttftMs} ms</td>
                  <td className="py-3 text-gray-700">{log.totalLatencyMs} ms</td>
                  <td className="py-3 text-gray-700">
                    {log.promptTokens} in / {log.completionTokens} out
                  </td>
                  <td className="py-3 font-bold text-gray-950">${log.costUsd.toFixed(6)}</td>
                  <td className="py-3 text-right">
                    <button className="text-blue-600 hover:underline text-xs font-sans font-medium cursor-pointer">
                      {t.buyer.logs.colInspect} →
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>

      {/* Drawer: Detailed Request Inspector */}
      {selectedLog && (
        <BCDrawer
          isOpen={isDrawerOpen}
          onClose={() => setIsDrawerOpen(false)}
          title={`Request ${selectedLog.id}`}
          subtitle={`${selectedLog.timestamp} • ${selectedLog.ipRegion}`}
        >
          <div className="space-y-6 text-xs">
            {/* Status Summary */}
            <div className="p-4 bg-gray-50 rounded-xl border border-gray-200/70 space-y-2">
              <div className="flex items-center justify-between">
                <span className="font-bold text-gray-900 font-mono">{t.buyer.logs.colStatus}</span>
                <BCBadge variant="success" size="md">{selectedLog.status} OK</BCBadge>
              </div>
              <div className="grid grid-cols-2 gap-3 pt-2 font-mono">
                <div>
                  <span className="text-gray-500 text-[10px] block">Time to First Token (TTFT)</span>
                  <span className="text-sm font-bold text-gray-900">{selectedLog.ttftMs} ms</span>
                </div>
                <div>
                  <span className="text-gray-500 text-[10px] block">Total Duration</span>
                  <span className="text-sm font-bold text-gray-900">{selectedLog.totalLatencyMs} ms</span>
                </div>
              </div>
            </div>

            {/* Token & Cost Breakdown */}
            <div className="space-y-2">
              <h4 className="font-bold text-gray-900 uppercase font-mono tracking-wider text-[11px]">
                {t.buyer.logs.colTokens} & {t.buyer.logs.colCost}
              </h4>
              <div className="p-4 bg-gray-50 rounded-xl border border-gray-100 space-y-2 font-mono">
                <div className="flex justify-between">
                  <span className="text-gray-600">Prompt Tokens:</span>
                  <span className="font-bold text-gray-900">{selectedLog.promptTokens}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Completion Tokens:</span>
                  <span className="font-bold text-gray-900">{selectedLog.completionTokens}</span>
                </div>
                <div className="flex justify-between pt-2 border-t border-gray-200/80">
                  <span className="text-gray-900 font-bold">Total Request Charge:</span>
                  <span className="font-bold text-emerald-700">${selectedLog.costUsd.toFixed(6)}</span>
                </div>
              </div>
            </div>

            {/* Hardware Attestation & Security Header */}
            <div className="space-y-2">
              <h4 className="font-bold text-gray-900 uppercase font-mono tracking-wider text-[11px]">
                Attestation & Security Enclave
              </h4>
              <div className="p-4 bg-emerald-50/60 rounded-xl border border-emerald-200/80 space-y-2 font-mono text-[11px] text-emerald-950">
                <div className="flex items-center gap-2">
                  <ShieldCheck className="w-4 h-4 text-emerald-600" />
                  <span className="font-bold">Cryptographic Receipt #RCPT-982A1-NITRO</span>
                </div>
                <p className="text-[11px] text-emerald-900 font-sans leading-relaxed">
                  Execution completed inside Confidential Computing enclave with zero telemetry retention.
                </p>
              </div>
            </div>

            {/* Request Headers */}
            <div className="space-y-2">
              <h4 className="font-bold text-gray-900 uppercase font-mono tracking-wider text-[11px]">
                Trace Metadata
              </h4>
              <pre className="p-3 bg-gray-950 text-gray-200 rounded-xl text-[10px] font-mono overflow-x-auto">
{JSON.stringify(
  {
    "x-burncloud-tier": selectedLog.tier.toLowerCase(),
    "x-burncloud-region": selectedLog.ipRegion,
    "x-burncloud-request-id": selectedLog.id,
    "x-ratelimit-remaining": 1198,
    "failover-count": selectedLog.failoverOccurred ? 1 : 0
  },
  null,
  2
)}
              </pre>
            </div>
          </div>
        </BCDrawer>
      )}
    </div>
  );
}
