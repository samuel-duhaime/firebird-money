from enum import Enum
from typing import Optional, List
from sqlmodel import SQLModel, Field
from sqlalchemy import Column, Integer
from sqlalchemy.dialects import postgresql


class CategoryType(str, Enum):
    expense = "expense"
    income = "income"
    transfer = "transfer"


class Category(SQLModel, table=True):
    __tablename__ = "categories"
    id: Optional[int] = Field(default=None, primary_key=True)
    account_id: int
    name: str
    type: CategoryType
    logo_url: Optional[str] = None
    tags: Optional[List[int]] = Field(
        default=None, sa_column=Column(postgresql.ARRAY(Integer), nullable=True)
    )

class CategoryCreate(SQLModel):
    account_id: int
    name: str
    type: CategoryType
    logo_url: Optional[str] = None
    tags: Optional[List[int]] = None


class CategoryPatch(SQLModel):
    name: Optional[str] = None
    type: Optional[CategoryType] = None
    logo_url: Optional[str] = None
    tags: Optional[List[int]] = None
