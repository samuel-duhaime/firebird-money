# FireBird Money

FireBird Money is a personal finance application that helps you manage your finances, track expenses, and create budgets. It is built using Python (FastAPI) for the backend and TypeScript (Next.js) for the frontend.

## Table of Contents

- [Getting Started](#getting-started)
- [Requirements](#requirements)
- [Backend](#backend)
- [Frontend](#frontend)
- [License](#license)

## Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/samuel-duhaime/firebird-money.git
   cd firebird-money
   ```
2. Follow the instructions below to set up the backend and frontend.

## Requirements

- Python 3.12+
- Node.js 22+
- PostgreSQL (locally or via Docker)

## Backend

### Setup

1. Copy the example environment file:
   ```bash
   cp backend/.env.example backend/.env
   ```
2. Install dependencies (using Poetry):
   ```bash
   cd backend
   poetry install
   ```

### Running

Start the backend server:

```bash
cd backend
poetry run start
```

### Testing

- **Run tests:**
  ```bash
  poetry run pytest -v
  ```

### Linting & Formatting

- **Lint:**
  ```bash
  poetry run ruff check app tests
  ```
- **Format code:**
  ```bash
  poetry run black app tests
  ```
- **Check formatting only:**
  ```bash
  poetry run black --check app tests
  ```

## Frontend

### Setup

Install dependencies:

```bash
cd frontend
npm install
```

### Running

Start the frontend development server:

```bash
cd frontend
npm run dev
```

The frontend will be available at [http://localhost:3000](http://localhost:3000).

### Testing & Linting

- **Lint:**
  ```bash
  npm run lint
  ```
- **Check formatting:**
  ```bash
  npm run format -- --check
  ```
- **Build:**
  ```bash
  npm run build
  ```

## Notes

- Make sure PostgreSQL is running and matches the credentials in `backend/.env`.
- For production, use `npm run build` and `npm start` in the frontend.

## License

See [LICENSE](LICENSE).
