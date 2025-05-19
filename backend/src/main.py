from fastapi import FastAPI

from src.features.accounts.routes import accounts_router

app = FastAPI()

app.include_router(accounts_router, prefix="/accounts")


def start():
    """Launched with `poetry run start`. Ensure this command is executed from the backend root directory."""
    import uvicorn

    uvicorn.run("src.main:app", host="0.0.0.0", port=8000, reload=True)
