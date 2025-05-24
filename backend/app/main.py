from fastapi import FastAPI

from app.features.accounts.routes import accounts_router
from app.features.users.routes import users_router

app = FastAPI()

app.include_router(accounts_router, prefix="/accounts")
app.include_router(users_router, prefix="/users")


def start():
    """Launched with `poetry run start`. Ensure this command is executed from the backend root directory."""
    import uvicorn

    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)
