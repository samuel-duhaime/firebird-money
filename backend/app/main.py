from fastapi import FastAPI

from app.features.accounts.routes import accounts_router
from app.features.users.routes import users_router
from app.db import init_db

app = FastAPI()

app.include_router(accounts_router, prefix="/accounts")
app.include_router(users_router, prefix="/users")


@app.on_event("startup")
async def startup_event():
    init_db()  # Initialize the database


def start():
    """Launched with `poetry run start`. Ensure this command is executed from the backend root directory."""
    import uvicorn

    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)
