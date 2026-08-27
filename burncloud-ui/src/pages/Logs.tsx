import React, { useState } from 'react';
import { Button, Card, Badge, Drawer, Input } from '@/components/ui';
import { MOCK_LOGS, Log } from '@/types';
import { Search, Filter, Clock, CheckCircle2, AlertCircle, AlertTriangle } from 'lucide-react';
import { motion } from 'motion/react';

export function Logs() {
  const [selectedLog, setSelectedLog] = useState<Log | null>(null);

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out h-full flex flex-col">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Logs</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Detailed observability into every routed request.</p>
        </div>
        <div className="flex items-center gap-3">
          <div className="relative w-72">
            <Search className="w-[15px] h-[15px] absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <Input placeholder="Search by request ID, customer, route..." className="pl-9 bg-white text-[13px]" />
          </div>
          <Button variant="secondary" className="gap-2 text-[13px]"><Filter className="w-4 h-4" /> Filter</Button>
        </div>
      </div>

      <Card className="flex-1 overflow-hidden flex flex-col">
        <div className="overflow-x-auto flex-1">
          <table className="w-full text-sm text-left">
            <thead className="text-[13px] text-gray-500 bg-transparent border-b border-gray-200/60">
              <tr>
                <th className="px-6 py-4 font-medium">Timestamp</th>
                <th className="px-6 py-4 font-medium">Request ID</th>
                <th className="px-6 py-4 font-medium">Customer</th>
                <th className="px-6 py-4 font-medium">Route / Model</th>
                <th className="px-6 py-4 font-medium">Status</th>
                <th className="px-6 py-4 font-medium text-right">Latency</th>
                <th className="px-6 py-4 font-medium text-right">Tokens</th>
                <th className="px-6 py-4 font-medium text-right">Cost</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {MOCK_LOGS.map((log) => (
                <tr 
                  key={log.id} 
                  className="hover:bg-gray-50/80 transition-colors cursor-pointer group"
                  onClick={() => setSelectedLog(log)}
                >
                  <td className="px-6 py-4 font-mono text-[13px] text-gray-500 whitespace-nowrap">{log.timestamp}</td>
                  <td className="px-6 py-4 font-mono text-[13px] text-gray-900">{log.requestId}</td>
                  <td className="px-6 py-4 text-[13px] text-gray-900">{log.customer}</td>
                  <td className="px-6 py-4">
                    <div className="text-[13px] text-gray-900 font-medium">{log.route}</div>
                    <div className="text-[12px] text-gray-500 mt-0.5">{log.model} <span className="text-gray-300 mx-1">•</span> {log.provider}</div>
                  </td>
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-2">
                      {log.status === 'Success' && <CheckCircle2 className="w-[15px] h-[15px] text-green-500" />}
                      {log.status === 'Fallback' && <AlertTriangle className="w-[15px] h-[15px] text-amber-500" />}
                      {log.status === 'Timeout' && <AlertCircle className="w-[15px] h-[15px] text-red-500" />}
                      <span className={`font-medium text-[13px] ${
                        log.status === 'Success' ? 'text-green-700' : 
                        log.status === 'Fallback' ? 'text-amber-700' : 'text-red-700'
                      }`}>{log.status}</span>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right tabular-nums text-[13px] text-gray-500">{log.latency.toLocaleString()}ms</td>
                  <td className="px-6 py-4 text-right tabular-nums text-[13px] text-gray-500">{log.tokens.toLocaleString()}</td>
                  <td className="px-6 py-4 text-right tabular-nums text-[13px] font-medium text-gray-900">${log.cost.toFixed(3)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      {/* Log Detail Drawer */}
      <Drawer isOpen={!!selectedLog} onClose={() => setSelectedLog(null)} title="Request Detail">
        {selectedLog && (
          <div className="p-6 space-y-8">
            {/* Meta */}
            <div className="flex items-center gap-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
              <div className="flex-1">
                <p className="text-xs text-gray-500 mb-1">Request ID</p>
                <p className="font-mono text-sm text-gray-900">{selectedLog.requestId}</p>
              </div>
              <div className="w-px h-8 bg-gray-200"></div>
              <div className="flex-1">
                <p className="text-xs text-gray-500 mb-1">Customer</p>
                <p className="text-sm font-medium text-gray-900">{selectedLog.customer}</p>
              </div>
              <div className="w-px h-8 bg-gray-200"></div>
              <div className="flex-1">
                <p className="text-xs text-gray-500 mb-1">Total Cost</p>
                <p className="text-sm font-medium text-gray-900">${selectedLog.cost.toFixed(3)}</p>
              </div>
            </div>

            {/* Timeline */}
            <div>
              <h3 className="text-sm font-medium text-gray-900 mb-4 flex items-center gap-2"><Clock className="w-4 h-4" /> Routing Timeline</h3>
              <div className="relative pl-4 space-y-6 border-l-2 border-gray-100 ml-2">
                <div className="relative">
                  <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-gray-300 ring-4 ring-white"></div>
                  <p className="text-sm text-gray-900">Request received</p>
                  <p className="text-xs text-gray-500 mt-1">{selectedLog.timestamp}</p>
                </div>
                <div className="relative">
                  <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-blue-400 ring-4 ring-white"></div>
                  <p className="text-sm text-gray-900">Matched route: <span className="font-medium">{selectedLog.route}</span></p>
                </div>
                <div className="relative">
                  <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-blue-400 ring-4 ring-white"></div>
                  <p className="text-sm text-gray-900">Selected primary model: <span className="font-medium">{selectedLog.status === 'Fallback' || selectedLog.status === 'Timeout' ? 'claude-fable-5' : selectedLog.model}</span></p>
                </div>
                
                {selectedLog.status === 'Timeout' && (
                  <>
                    <div className="relative">
                      <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-red-400 ring-4 ring-white"></div>
                      <p className="text-sm text-red-600 font-medium">Timeout after 10s</p>
                    </div>
                    <div className="relative">
                      <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-yellow-400 ring-4 ring-white"></div>
                      <p className="text-sm text-gray-900">Triggered fallback condition: <span className="font-mono text-xs">Timeout {">"} 8s</span></p>
                    </div>
                    <div className="relative">
                      <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-blue-400 ring-4 ring-white"></div>
                      <p className="text-sm text-gray-900">Retried with <span className="font-medium">{selectedLog.fallbackTo}</span></p>
                    </div>
                  </>
                )}
                
                {selectedLog.status === 'Fallback' && (
                  <>
                    <div className="relative">
                      <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-yellow-400 ring-4 ring-white"></div>
                      <p className="text-sm text-yellow-700 font-medium">Provider error rate exceeded threshold</p>
                    </div>
                    <div className="relative">
                      <div className="absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full bg-blue-400 ring-4 ring-white"></div>
                      <p className="text-sm text-gray-900">Falling back to <span className="font-medium">{selectedLog.fallbackTo}</span></p>
                    </div>
                  </>
                )}

                <div className="relative">
                  <div className={`absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full ring-4 ring-white ${selectedLog.status === 'Timeout' ? 'bg-red-500' : 'bg-green-500'}`}></div>
                  <p className="text-sm font-medium text-gray-900">Response completed in {selectedLog.latency}ms</p>
                </div>
              </div>
            </div>

            {/* Prompt Preview */}
            <div>
              <h3 className="text-sm font-medium text-gray-900 mb-2">Prompt Snippet</h3>
              <div className="bg-gray-900 rounded-xl p-4 text-gray-300 font-mono text-sm overflow-hidden">
                <div className="truncate">"system": "You are a senior legal..."</div>
                <div className="truncate mt-2">"user": "Summarize the following contract and..."</div>
              </div>
            </div>
          </div>
        )}
      </Drawer>
    </div>
  );
}
