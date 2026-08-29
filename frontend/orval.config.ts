import { defineConfig } from 'orval'

// Regenerate whenever the backend contract changes: `npm run generate:api`
// (requires the backend running on devMachine, reachable at localhost:8081
// from the same machine — see AGENTS.md's "Frontend" section).
export default defineConfig({
  api: {
    input: 'http://localhost:8081/api/openapi.json',
    output: {
      mode: 'tags-split',
      target: 'src/api/generated',
      client: 'react-query',
      httpClient: 'fetch',
      clean: true,
      override: {
        mutator: {
          path: 'src/api/mutator/customFetch.ts',
          name: 'customFetch',
        },
        query: {
          useQuery: true,
        },
      },
    },
  },
})
