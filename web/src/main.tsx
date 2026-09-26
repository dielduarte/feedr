import '@fontsource-variable/inter'
import './index.css'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Toaster } from '@/components/ui/sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import { App } from './App'

declare global {
  interface Window {
    __FEEDRSAUROS_TRAFFIC_LIGHTS__?: boolean
  }
}

// The desktop app keeps this attribute current as the window enters and leaves fullscreen.
if (window.__FEEDRSAUROS_TRAFFIC_LIGHTS__) document.documentElement.dataset.trafficLights = ''

const client = new QueryClient({
  defaultOptions: {
    // Live updates arrive over SSE, so background refetching only needs to catch up occasionally.
    queries: { staleTime: 30_000, retry: 1 },
  },
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={client}>
      <TooltipProvider delayDuration={400}>
        <App />
        <Toaster position="bottom-center" />
      </TooltipProvider>
    </QueryClientProvider>
  </StrictMode>,
)
