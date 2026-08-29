import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { useLogin } from '../../api/generated/auth/auth'
import { useRefreshCurrentUser } from './useCurrentUser'

export function LoginPage() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const navigate = useNavigate()
  const refreshCurrentUser = useRefreshCurrentUser()
  const login = useLogin()

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    const result = await login.mutateAsync({ data: { email, password } })
    if (result.status === 200) {
      await refreshCurrentUser()
      navigate('/', { replace: true })
    } else {
      setError(result.data.error)
    }
  }

  return (
    <div className="auth-page">
      <form className="auth-form" onSubmit={handleSubmit}>
        <h1>Log in</h1>
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
        <p className="auth-switch">
          No account? <Link to="/signup">Sign up</Link>
        </p>
      </form>
    </div>
  )
}
