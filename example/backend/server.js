import express from 'express'
import cors from 'cors'
import mysql from 'mysql2/promise'
import { createClient } from 'redis'

const app = express()

app.use(cors())
app.use(express.json())

const db = await mysql.createPool({
    host: process.env.DB_HOST,
    port: Number(process.env.DB_PORT),
    user: process.env.DB_USER,
    password: process.env.DB_PASSWORD,
    database: process.env.DB_NAME
})

const redis = createClient({
    socket: {
        host: process.env.REDIS_HOST,
        port: Number(process.env.REDIS_PORT)
    }
})

await redis.connect()

await db.query(`
  CREATE TABLE IF NOT EXISTS tasks (
    id INT AUTO_INCREMENT PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    done BOOLEAN NOT NULL DEFAULT false,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
  )
`)

app.get('/health', async (req, res) => {
    await redis.set('last_healthcheck', new Date().toISOString())
    res.json({ ok: true })
})

app.get('/tasks', async (req, res) => {
    const [tasks] = await db.query('SELECT * FROM tasks ORDER BY id DESC')
    res.json(tasks)
})

app.post('/tasks', async (req, res) => {
    const title = String(req.body.title ?? '').trim()

    if (!title) {
        return res.status(400).json({ error: 'Title is required' })
    }

    const [result] = await db.query(
        'INSERT INTO tasks (title) VALUES (?)',
        [title]
    )

    await redis.del('tasks_count')

    res.status(201).json({
        id: result.insertId,
        title,
        done: false
    })
})

app.patch('/tasks/:id/toggle', async (req, res) => {
    await db.query(
        'UPDATE tasks SET done = NOT done WHERE id = ?',
        [req.params.id]
    )

    res.json({ ok: true })
})

app.delete('/tasks/:id', async (req, res) => {
    await db.query('DELETE FROM tasks WHERE id = ?', [req.params.id])
    await redis.del('tasks_count')

    res.json({ ok: true })
})

app.get('/stats', async (req, res) => {
    let count = await redis.get('tasks_count')

    if (!count) {
        const [[row]] = await db.query('SELECT COUNT(*) AS count FROM tasks')
        count = String(row.count)
        await redis.set('tasks_count', count, { EX: 30 })
    }

    res.json({
        tasksCount: Number(count),
        cached: true
    })
})

app.listen(4000, () => {
    console.log('Backend running on http://localhost:4000')
})