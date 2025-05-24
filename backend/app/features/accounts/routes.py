from fastapi import APIRouter, HTTPException, Depends
from sqlmodel import Session
from app.db import get_session
from .models import (
    Account,
    AccountStatus,
    AccountCreate,
    AccountPatch,
)

accounts_router = APIRouter()


@accounts_router.get("/{id}", response_model=Account)
def get_account(id: int, session: Session = Depends(get_session)):
    account = session.get(Account, id)
    if not account:
        raise HTTPException(status_code=404, detail="Account not found")
    return account


@accounts_router.post("/", response_model=Account)
def post_account(account: AccountCreate, session: Session = Depends(get_session)):
    acc = Account(status=AccountStatus.pending, **account.model_dump())
    session.add(acc)
    session.commit()
    session.refresh(acc)
    return acc


@accounts_router.patch("/{id}", response_model=Account)
def patch_account(
    id: int, fields: AccountPatch, session: Session = Depends(get_session)
):
    account = session.get(Account, id)
    if not account:
        raise HTTPException(status_code=404, detail="Account not found")
    for k, v in fields.model_dump(exclude_unset=True).items():
        setattr(account, k, v)
    session.add(account)
    session.commit()
    session.refresh(account)
    return account


# TODO: Implement a better response for delete
@accounts_router.delete("/{id}")
def delete_account(id: int, session: Session = Depends(get_session)):
    account = session.get(Account, id)
    if not account:
        raise HTTPException(status_code=404, detail="Account not found")
    session.delete(account)
    session.commit()
    return {"ok": True}
