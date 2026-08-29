import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import {
  getListTenantsQueryKey,
  listTenants,
  usePlatformLogin,
} from '../../api/generated/platform/platform'

export function PlatformLoginPage() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const login = usePlatformLogin()

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await login.mutateAsync({ data: { email, password } })
    if (result.status === 200) {
      const tenants = await listTenants()
      queryClient.setQueryData(getListTenantsQueryKey(), tenants)
      navigate('/platform', { replace: true })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div className="auth-page">
      <form className="auth-form" onSubmit={handleSubmit}>
        <h1>Platform admin login</h1>
        {error && <p className="form-error">{error}</p>}
        <label>
          Email
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            autoFocus
          />
        </label>
        <label>
          Password
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </label>
        <button type="submit" disabled={login.isPending}>
          {login.isPending ? 'Logging in…' : 'Log in'}
        </button>
      </form>
    </div>
  )
}
