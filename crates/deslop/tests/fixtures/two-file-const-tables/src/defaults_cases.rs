// Test-data blobs for the defaults suite: a generated DTO, a migration
// schema, HTTP routes and a config document. Unrelated to the payloads
// in `boilerplate_cases.rs` in language, subject and ownership alike.

const GENERATED_DTO: &str = r"
namespace Contracts.Generated;

public partial record CustomerDto(
    Guid Id,
    string DisplayName,
    string EmailAddress,
    DateTimeOffset RegisteredOn);
";

const ALEMBIC_INITIAL_SCHEMA: &str = r"
revision = '0001_initial'
down_revision = None

def upgrade():
    op.create_table(
        'customer',
        sa.Column('id', sa.Uuid(), primary_key=True),
        sa.Column('display_name', sa.String(200), nullable=False),
    )
";

const FASTAPI_ROUTES: &str = r"
@router.get('/customers/{customer_id}')
async def read_customer(customer_id: UUID, session: Session = Depends(get_session)):
    customer = await session.get(Customer, customer_id)
    if customer is None:
        raise HTTPException(status_code=404)
    return customer
";

const OPENAPI_DOCUMENT: &str = r"
openapi: 3.1.0
info:
  title: Billing
  version: 2.4.0
paths:
  /invoices:
    get:
      operationId: listInvoices
      responses:
        '200':
          description: ok
";

const TERRAFORM_STACK: &str = r"
resource \"aws_s3_bucket\" \"reports\" {
  bucket = \"nimblesite-reports\"
  tags = {
    Environment = \"production\"
  }
}
";

const SQL_SEED: &str = r"
INSERT INTO tariff (code, band, rate) VALUES
  ('STD', 1, 0.20),
  ('RED', 2, 0.05),
  ('ZER', 3, 0.00);
";
