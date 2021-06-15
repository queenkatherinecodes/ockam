defmodule Ockam.Stream.Client.BiDirectional.PublisherProxy do
  @moduledoc """
  Publisher proxy worker to add return_stream and encode Ockam messages to binary
  Uses internal Stream.Client.Publisher through which it sends messages
  """
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Stream.Client.Publisher

  require Logger

  @impl true
  def setup(options, state) do
    consumer_stream = Keyword.fetch!(options, :consumer_stream)
    publisher_stream = Keyword.fetch!(options, :publisher_stream)
    stream_options = Keyword.fetch!(options, :stream_options)

    send(self(), {:init, publisher_stream, stream_options})

    {:ok, Map.merge(state, %{consumer_stream: consumer_stream})}
  end

  @impl true
  def handle_message({:init, publisher_stream, stream_options}, state) do
    {:ok, publisher_address} =
      Publisher.create(Keyword.merge(stream_options, stream_name: publisher_stream))

    {:ok, Map.put(state, :publisher_address, publisher_address)}
  end

  def handle_message(%{payload: _} = message, %{publisher_address: _} = state) do
    %{
      consumer_stream: consumer_stream,
      address: self_address,
      publisher_address: publisher_address
    } = state

    [^self_address | onward_route] = Message.onward_route(message)
    forwarded_message = %{message | onward_route: onward_route}

    {message_id, state} = next_message_id(state)

    Logger.debug("Forward message #{inspect(forwarded_message)} with id #{inspect(message_id)}")

    encoded_message =
      Ockam.Stream.Client.BiDirectional.encode_message(%{
        message: forwarded_message,
        return_stream: consumer_stream,
        message_id: message_id
      })

    binary_message =
      Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, encoded_message)

    Ockam.Router.route(%{
      payload: binary_message,
      onward_route: [publisher_address],
      return_route: []
    })

    {:ok, state}
  end

  def handle_message(%{payload: _} = message, state) do
    ## Delay message processing
    send(self(), message)
    {:ok, state}
  end

  def next_message_id(state) do
    current = Map.get(state, :message_id, 0)
    next = current + 1
    {next, Map.put(state, :message_id, next)}
  end
end
