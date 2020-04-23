package util

import enumeratum.{EnumEntry, Enum => Enumeratum}

sealed trait ConnectorCapability extends EnumEntry

object ConnectorCapability extends Enumeratum[ConnectorCapability] {
  val values = findValues

  object ScalarListsCapability            extends ConnectorCapability
  object EmbeddedTypesCapability          extends ConnectorCapability
  object JoinRelationsFilterCapability    extends ConnectorCapability
  object TransactionalExecutionCapability extends ConnectorCapability

  object SupportsExistingDatabasesCapability extends ConnectorCapability
  object MigrationsCapability                extends ConnectorCapability
  object RawAccessCapability                 extends ConnectorCapability
  object IntrospectionCapability             extends ConnectorCapability
  object JoinRelationLinksCapability         extends ConnectorCapability // the ability to join using relation links
  object RelationLinkListCapability          extends ConnectorCapability // relation links can be stored inline in a node in a list
  object RelationLinkTableCapability         extends ConnectorCapability // relation links are stored in a table
  object EnumCapability                      extends ConnectorCapability // supports native enums

  sealed trait IdCapability   extends ConnectorCapability
  object IntIdCapability      extends IdCapability
  object UuidIdCapability     extends IdCapability
  object IdSequenceCapability extends IdCapability

  object Prisma2Capability extends ConnectorCapability
}

case class ConnectorCapabilities(capabilities: Set[ConnectorCapability]) {
  def has(capability: ConnectorCapability): Boolean    = capabilities.contains(capability)
  def hasNot(capability: ConnectorCapability): Boolean = !has(capability)
}

object ConnectorCapabilities {
  import ConnectorCapability._

  val empty: ConnectorCapabilities                                     = ConnectorCapabilities(Set.empty[ConnectorCapability])
  def apply(capabilities: ConnectorCapability*): ConnectorCapabilities = ConnectorCapabilities(Set(capabilities: _*))

  lazy val sqlite: ConnectorCapabilities   = ConnectorCapabilities(sqlShared)
  lazy val postgres: ConnectorCapabilities = ConnectorCapabilities(sqlShared + ScalarListsCapability + EnumCapability)
  lazy val mysql: ConnectorCapabilities    = ConnectorCapabilities(sqlShared + EnumCapability)

  private lazy val sqlShared: Set[ConnectorCapability] = {
    Set(
      TransactionalExecutionCapability,
      JoinRelationsFilterCapability,
      JoinRelationLinksCapability,
      RelationLinkTableCapability,
      MigrationsCapability,
      IntrospectionCapability,
      SupportsExistingDatabasesCapability,
      IntIdCapability,
      RawAccessCapability,
      IdSequenceCapability,
      Prisma2Capability,
      UuidIdCapability
    )
  }
}
